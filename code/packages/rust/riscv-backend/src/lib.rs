//! RV32I backend for the typed CIR stage of the LANG pipeline.
//!
//! This backend deliberately consumes `CIRInstr`, never dynamic IIR.  It is a
//! small but executable scalar lane: supported functions lower to real RV32I
//! bytes and `run_binary` executes those bytes in the in-tree simulator.

use std::collections::{HashMap, HashSet};
use std::fmt;

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use riscv_encoder::{
    assemble, encode_add, encode_addi, encode_and, encode_andi, encode_beq, encode_bne,
    encode_ecall, encode_jal, encode_lui, encode_or, encode_sll, encode_slt, encode_sltu,
    encode_sra, encode_srai, encode_srl, encode_sub, encode_xor, encode_xori, A0, RET_WORD,
    X0_ZERO, X1_RA,
};
use riscv_simulator::RiscVSimulator;
use vm_core::value::Value;

const DEFAULT_MEMORY_SIZE: usize = 64 * 1024;
const DEFAULT_STEP_LIMIT: usize = 100_000;
const ARG_REGISTERS: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
const A1: u32 = 11;
const SCRATCH_REGISTER: u32 = 31;
const SECOND_SCRATCH_REGISTER: u32 = 27;
const VALUE_REGISTERS: [u32; 6] = [5, 6, 7, 28, 29, 30];

/// The RV32I backend.  Stateless — every compilation gets fresh allocation.
#[derive(Debug, Default, Clone, Copy)]
pub struct Riscv32Backend;

impl Riscv32Backend {
    pub fn new() -> Self {
        Self
    }
}

/// Result of running a compiled RV32I function in the in-tree simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunResult {
    pub return_value: i32,
    pub return_value_high: u32,
    pub halted: bool,
    pub steps: usize,
}

/// Errors reported by the RISC-V scalar lowering and execution surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    UnsupportedOp(String),
    UnsupportedType(String),
    InvalidOperand(String),
    UndefinedVariable(String),
    UndefinedLabel(String),
    ImmediateOutOfRange(i64),
    OutOfRegisters,
    TooManyArguments(usize),
    BranchOutOfRange { label: String, offset: i64 },
    ExecutionDidNotHalt { steps: usize },
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOp(op) => write!(f, "riscv-backend: unsupported op {op:?}"),
            Self::UnsupportedType(ty) => {
                write!(f, "riscv-backend: unsupported RV32I scalar type {ty:?}")
            }
            Self::InvalidOperand(detail) => write!(f, "riscv-backend: invalid operand: {detail}"),
            Self::UndefinedVariable(name) => {
                write!(f, "riscv-backend: undefined variable {name:?}")
            }
            Self::UndefinedLabel(name) => write!(f, "riscv-backend: undefined label {name:?}"),
            Self::ImmediateOutOfRange(value) => {
                write!(f, "riscv-backend: {value} does not fit in an RV32I integer")
            }
            Self::OutOfRegisters => write!(
                f,
                "riscv-backend: scalar temporary-register pool exhausted (max {})",
                VALUE_REGISTERS.len()
            ),
            Self::TooManyArguments(count) => write!(
                f,
                "riscv-backend: {count} arguments exceed the RV32I starter ABI limit of {}",
                ARG_REGISTERS.len()
            ),
            Self::BranchOutOfRange { label, offset } => write!(
                f,
                "riscv-backend: branch to label {label:?} has out-of-range offset {offset}"
            ),
            Self::ExecutionDidNotHalt { steps } => write!(
                f,
                "riscv-backend: simulator did not halt within {steps} steps"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Lower a single typed CIR function to a flat little-endian RV32I binary.
pub fn compile(ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    let mut lowerer = Lowerer::new(ctx)?;
    for instr in cir {
        lowerer.lower(instr)?;
    }
    lowerer.resolve_branches()?;
    if lowerer.words.is_empty() {
        lowerer.words.push(RET_WORD);
    }
    Ok(assemble(&lowerer.words))
}

/// Run a function binary under the starter RV32I ABI.
///
/// The input binary ends in a normal `ret`.  The runner appends a single
/// `ecall` trampoline, initializes `ra` to it, and starts the function at
/// address zero.  This preserves normal function code while giving a flat
/// binary a deterministic simulator exit point.
pub fn run_binary(binary: &[u8], args: &[Value]) -> Result<RunResult, BackendError> {
    if args.len() > ARG_REGISTERS.len() {
        return Err(BackendError::TooManyArguments(args.len()));
    }

    let mut program = binary.to_vec();
    let return_trampoline = program.len();
    program.extend_from_slice(&encode_ecall().to_le_bytes());

    let mut simulator = RiscVSimulator::new(DEFAULT_MEMORY_SIZE);
    simulator.load_program(&program);
    simulator
        .regs
        .write(X1_RA as usize, return_trampoline as u32);
    simulator.regs.write(2, (DEFAULT_MEMORY_SIZE - 16) as u32);
    for (index, value) in args.iter().enumerate() {
        simulator
            .regs
            .write(ARG_REGISTERS[index] as usize, value_to_rv32(value)?);
    }

    let result = simulator.run_loaded_with_limit(DEFAULT_STEP_LIMIT);
    if !result.halted {
        return Err(BackendError::ExecutionDidNotHalt {
            steps: result.steps,
        });
    }
    Ok(RunResult {
        return_value: simulator.regs.read(A0 as usize) as i32,
        return_value_high: simulator.regs.read(A1 as usize),
        halted: result.halted,
        steps: result.steps,
    })
}

struct Lowerer {
    words: Vec<u32>,
    env: Vec<(String, ValueLocation)>,
    /// Values known to fit in one RV32 register despite an `i64`/`u64` CIR type.
    word_sized_values: HashSet<String>,
    labels: HashMap<String, usize>,
    branches: Vec<PendingBranch>,
    next_value_register: usize,
    next_internal_label: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueLocation {
    Word(u32),
    Pair { lo: u32, hi: u32 },
}

impl ValueLocation {
    fn low(self) -> u32 {
        match self {
            Self::Word(register) | Self::Pair { lo: register, .. } => register,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BranchKind {
    EqZero { rs1: u32 },
    NeZero { rs1: u32 },
    Jump,
}

#[derive(Debug, Clone)]
struct PendingBranch {
    word_index: usize,
    label: String,
    kind: BranchKind,
}

impl Lowerer {
    fn new(ctx: &FunctionContext<'_>) -> Result<Self, BackendError> {
        if ctx.params.len() > ARG_REGISTERS.len() {
            return Err(BackendError::TooManyArguments(ctx.params.len()));
        }
        let mut env = Vec::with_capacity(ctx.params.len());
        let mut next_argument = 0;
        for (name, ty) in ctx.params {
            if !is_rv32_value_type(ty) {
                return Err(BackendError::UnsupportedType(ty.clone()));
            }
            let location = if matches!(ty.as_str(), "i64" | "u64") {
                if next_argument + 1 >= ARG_REGISTERS.len() {
                    return Err(BackendError::TooManyArguments(ctx.params.len()));
                }
                let pair = ValueLocation::Pair {
                    lo: ARG_REGISTERS[next_argument],
                    hi: ARG_REGISTERS[next_argument + 1],
                };
                next_argument += 2;
                pair
            } else {
                if next_argument >= ARG_REGISTERS.len() {
                    return Err(BackendError::TooManyArguments(ctx.params.len()));
                }
                let word = ValueLocation::Word(ARG_REGISTERS[next_argument]);
                next_argument += 1;
                word
            };
            env.push((name.clone(), location));
        }
        Ok(Self {
            words: Vec::new(),
            env,
            word_sized_values: HashSet::new(),
            labels: HashMap::new(),
            branches: Vec::new(),
            next_value_register: 0,
            next_internal_label: 0,
        })
    }

    fn lower(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        let op = instr.op.as_str();
        if op == "ret_void" {
            self.words.push(RET_WORD);
            return Ok(());
        }
        if let Some(ty) = op.strip_prefix("ret_") {
            self.require_scalar_type(ty)?;
            match self.var_location(instr, 0, op)? {
                ValueLocation::Word(src) => self.words.push(encode_addi(A0, src, 0)),
                ValueLocation::Pair { lo, hi } => {
                    self.words.push(encode_addi(A0, lo, 0));
                    self.words.push(encode_addi(A1, hi, 0));
                }
            }
            self.words.push(RET_WORD);
            return Ok(());
        }
        if let Some(ty) = op.strip_prefix("const_") {
            self.require_scalar_type(ty)?;
            if matches!(ty, "i64" | "u64") && !wide_literal_fits_word(instr.srcs.first(), ty)? {
                let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
                    unreachable!("dest_pair always returns a pair")
                };
                let (low, high) = wide_literal_words(instr.srcs.first())?;
                self.load_constant(lo, low);
                self.load_constant(hi, high);
            } else {
                let rd = self.dest(instr, op)?;
                self.load_constant(rd, literal_word(instr.srcs.first(), ty)?);
                self.mask_unsigned(rd, ty);
            }
            if matches!(ty, "i64" | "u64") && wide_literal_fits_word(instr.srcs.first(), ty)? {
                self.word_sized_values.insert(
                    instr
                        .dest
                        .as_ref()
                        .expect("const_* destinations are required above")
                        .clone(),
                );
            }
            return Ok(());
        }

        if op == "label" {
            let label = self.label_src(instr, 0, op)?;
            self.labels.insert(label, self.words.len() * 4);
            return Ok(());
        }

        if op == "jmp" {
            self.record_branch(instr, 0, BranchKind::Jump, op)?;
            return Ok(());
        }

        if matches!(op, "jmp_if_false" | "br_false_bool") {
            let condition = self.var_src(instr, 0, op)?;
            self.record_branch(instr, 1, BranchKind::EqZero { rs1: condition }, op)?;
            return Ok(());
        }

        if matches!(op, "jmp_if_true" | "br_true_bool") {
            let condition = self.var_src(instr, 0, op)?;
            self.record_branch(instr, 1, BranchKind::NeZero { rs1: condition }, op)?;
            return Ok(());
        }

        for family in ["add", "sub", "and", "or", "xor", "shl", "shr"] {
            if let Some(ty) = op.strip_prefix(&format!("{family}_")) {
                if matches!(ty, "i64" | "u64") {
                    return match family {
                        "add" => self.lower_wide_add(instr, op, is_signed(ty)),
                        "sub" => self.lower_wide_sub(instr, op, is_signed(ty)),
                        _ => Err(BackendError::UnsupportedType(ty.to_owned())),
                    };
                }
                self.require_operation_type(ty)?;
                let rd = self.dest(instr, op)?;
                let lhs = self.var_src(instr, 0, op)?;
                let rhs = self.var_src(instr, 1, op)?;
                let word = match family {
                    "add" => encode_add(rd, lhs, rhs),
                    "sub" => encode_sub(rd, lhs, rhs),
                    "and" => encode_and(rd, lhs, rhs),
                    "or" => encode_or(rd, lhs, rhs),
                    "xor" => encode_xor(rd, lhs, rhs),
                    "shl" => encode_sll(rd, lhs, rhs),
                    "shr" if is_signed(ty) => encode_sra(rd, lhs, rhs),
                    "shr" => encode_srl(rd, lhs, rhs),
                    _ => unreachable!(),
                };
                self.words.push(word);
                self.mask_unsigned(rd, ty);
                return Ok(());
            }
        }

        for family in ["neg", "not"] {
            if let Some(ty) = op.strip_prefix(&format!("{family}_")) {
                self.require_operation_type(ty)?;
                let rd = self.dest(instr, op)?;
                let src = self.var_src(instr, 0, op)?;
                self.words.push(match family {
                    "neg" => encode_sub(rd, X0_ZERO, src),
                    "not" => encode_xori(rd, src, -1),
                    _ => unreachable!(),
                });
                self.mask_unsigned(rd, ty);
                return Ok(());
            }
        }

        if let Some((relation, ty)) = comparison_parts(op) {
            if matches!(ty, "i64" | "u64") {
                return self.lower_wide_comparison(instr, op, relation, is_signed(ty));
            }
            self.require_comparison_type(instr, ty, op)?;
            let rd = self.dest(instr, op)?;
            let lhs = self.var_src(instr, 0, op)?;
            let rhs = self.var_src(instr, 1, op)?;
            let signed = is_signed(ty);
            match relation {
                "eq" => {
                    self.words.push(encode_xor(SCRATCH_REGISTER, lhs, rhs));
                    self.words
                        .push(riscv_encoder::encode_sltiu(rd, SCRATCH_REGISTER, 1));
                }
                "ne" => {
                    self.words.push(encode_xor(SCRATCH_REGISTER, lhs, rhs));
                    self.words.push(encode_sltu(rd, X0_ZERO, SCRATCH_REGISTER));
                }
                "lt" => self.words.push(if signed {
                    encode_slt(rd, lhs, rhs)
                } else {
                    encode_sltu(rd, lhs, rhs)
                }),
                "gt" => self.words.push(if signed {
                    encode_slt(rd, rhs, lhs)
                } else {
                    encode_sltu(rd, rhs, lhs)
                }),
                "le" => {
                    self.words.push(if signed {
                        encode_slt(SCRATCH_REGISTER, rhs, lhs)
                    } else {
                        encode_sltu(SCRATCH_REGISTER, rhs, lhs)
                    });
                    self.words.push(encode_xori(rd, SCRATCH_REGISTER, 1));
                }
                "ge" => {
                    self.words.push(if signed {
                        encode_slt(SCRATCH_REGISTER, lhs, rhs)
                    } else {
                        encode_sltu(SCRATCH_REGISTER, lhs, rhs)
                    });
                    self.words.push(encode_xori(rd, SCRATCH_REGISTER, 1));
                }
                _ => return Err(BackendError::UnsupportedOp(op.to_string())),
            }
            return Ok(());
        }

        Err(BackendError::UnsupportedOp(op.to_string()))
    }

    fn resolve_branches(&mut self) -> Result<(), BackendError> {
        for branch in &self.branches {
            let target = self
                .labels
                .get(&branch.label)
                .copied()
                .ok_or_else(|| BackendError::UndefinedLabel(branch.label.clone()))?;
            let offset = target as i64 - (branch.word_index * 4) as i64;
            let word = match branch.kind {
                BranchKind::EqZero { rs1 } => {
                    Self::check_branch_offset(&branch.label, offset, 4096)?;
                    encode_beq(rs1, X0_ZERO, offset as i32)
                }
                BranchKind::NeZero { rs1 } => {
                    Self::check_branch_offset(&branch.label, offset, 4096)?;
                    encode_bne(rs1, X0_ZERO, offset as i32)
                }
                BranchKind::Jump => {
                    Self::check_branch_offset(&branch.label, offset, 1 << 20)?;
                    encode_jal(X0_ZERO, offset as i32)
                }
            };
            self.words[branch.word_index] = word;
        }
        Ok(())
    }

    fn check_branch_offset(label: &str, offset: i64, max: i64) -> Result<(), BackendError> {
        if offset < -max || offset >= max {
            return Err(BackendError::BranchOutOfRange {
                label: label.to_owned(),
                offset,
            });
        }
        Ok(())
    }

    fn record_branch(
        &mut self,
        instr: &CIRInstr,
        label_index: usize,
        kind: BranchKind,
        op: &str,
    ) -> Result<(), BackendError> {
        let label = self.label_src(instr, label_index, op)?;
        self.branches.push(PendingBranch {
            word_index: self.words.len(),
            label,
            kind,
        });
        self.words.push(0);
        Ok(())
    }

    fn record_named_branch(&mut self, label: String, kind: BranchKind) {
        self.branches.push(PendingBranch {
            word_index: self.words.len(),
            label,
            kind,
        });
        self.words.push(0);
    }

    fn mark_label(&mut self, label: String) {
        self.labels.insert(label, self.words.len() * 4);
    }

    fn internal_label(&mut self, suffix: &str) -> String {
        let label = format!(".__riscv_wide_cmp_{}_{}", self.next_internal_label, suffix);
        self.next_internal_label += 1;
        label
    }

    fn dest(&mut self, instr: &CIRInstr, op: &str) -> Result<u32, BackendError> {
        let name = instr
            .dest
            .as_deref()
            .ok_or_else(|| BackendError::InvalidOperand(format!("{op} requires a dest")))?;
        self.allocate(name)
    }

    fn dest_pair(&mut self, instr: &CIRInstr, op: &str) -> Result<ValueLocation, BackendError> {
        let name = instr
            .dest
            .as_deref()
            .ok_or_else(|| BackendError::InvalidOperand(format!("{op} requires a dest")))?;
        self.allocate_pair(name)
    }

    fn var_src(&self, instr: &CIRInstr, index: usize, op: &str) -> Result<u32, BackendError> {
        let name = match instr.srcs.get(index) {
            Some(CIROperand::Var(name)) => name,
            _ => {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} srcs[{index}] must be Var"
                )))
            }
        };
        self.lookup(name)
    }

    fn var_location(
        &self,
        instr: &CIRInstr,
        index: usize,
        op: &str,
    ) -> Result<ValueLocation, BackendError> {
        let name = match instr.srcs.get(index) {
            Some(CIROperand::Var(name)) => name,
            _ => {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} srcs[{index}] must be Var"
                )))
            }
        };
        self.lookup_location(name)
    }

    fn label_src(&self, instr: &CIRInstr, index: usize, op: &str) -> Result<String, BackendError> {
        match instr.srcs.get(index) {
            Some(CIROperand::Var(name)) => Ok(name.clone()),
            _ => Err(BackendError::InvalidOperand(format!(
                "{op} srcs[{index}] must be a label Var"
            ))),
        }
    }

    fn allocate(&mut self, name: &str) -> Result<u32, BackendError> {
        if let Some((_, location)) = self.env.iter().find(|(existing, _)| existing == name) {
            return match location {
                ValueLocation::Word(reg) => Ok(*reg),
                ValueLocation::Pair { .. } => Err(BackendError::InvalidOperand(format!(
                    "{name:?} is already bound as a 64-bit value"
                ))),
            };
        }
        if self.next_value_register >= VALUE_REGISTERS.len() {
            return Err(BackendError::OutOfRegisters);
        }
        let reg = VALUE_REGISTERS[self.next_value_register];
        self.next_value_register += 1;
        self.env.push((name.to_owned(), ValueLocation::Word(reg)));
        Ok(reg)
    }

    fn allocate_pair(&mut self, name: &str) -> Result<ValueLocation, BackendError> {
        if let Some((_, location)) = self.env.iter().find(|(existing, _)| existing == name) {
            return match location {
                ValueLocation::Pair { lo, hi } => Ok(ValueLocation::Pair { lo: *lo, hi: *hi }),
                ValueLocation::Word(_) => Err(BackendError::InvalidOperand(format!(
                    "{name:?} is already bound as a 32-bit value"
                ))),
            };
        }
        if self.next_value_register + 1 >= VALUE_REGISTERS.len() {
            return Err(BackendError::OutOfRegisters);
        }
        let location = ValueLocation::Pair {
            lo: VALUE_REGISTERS[self.next_value_register],
            hi: VALUE_REGISTERS[self.next_value_register + 1],
        };
        self.next_value_register += 2;
        self.env.push((name.to_owned(), location));
        Ok(location)
    }

    fn lookup(&self, name: &str) -> Result<u32, BackendError> {
        match self.lookup_location(name)? {
            ValueLocation::Word(reg) => Ok(reg),
            ValueLocation::Pair { .. } => Err(BackendError::InvalidOperand(format!(
                "{name:?} is a 64-bit value where a 32-bit value is required"
            ))),
        }
    }

    fn lookup_location(&self, name: &str) -> Result<ValueLocation, BackendError> {
        self.env
            .iter()
            .find_map(|(existing, location)| (existing == name).then_some(*location))
            .ok_or_else(|| BackendError::UndefinedVariable(name.to_owned()))
    }

    fn lower_wide_add(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        signed: bool,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        let lhs = self.var_location(instr, 0, op)?;
        let rhs = self.var_location(instr, 1, op)?;
        let lhs_lo = lhs.low();
        self.words.push(encode_add(lo, lhs_lo, rhs.low()));
        self.words.push(encode_sltu(SCRATCH_REGISTER, lo, lhs_lo));
        self.copy_or_extend_high(hi, lhs, signed);
        self.words.push(encode_add(hi, hi, SCRATCH_REGISTER));
        self.add_or_extend_high(hi, rhs, signed);
        Ok(())
    }

    fn lower_wide_sub(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        signed: bool,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        let lhs = self.var_location(instr, 0, op)?;
        let rhs = self.var_location(instr, 1, op)?;
        let lhs_lo = lhs.low();
        self.words.push(encode_sub(lo, lhs_lo, rhs.low()));
        self.copy_or_extend_high(hi, lhs, signed);
        self.sub_or_extend_high(hi, rhs, signed);
        self.words
            .push(encode_sltu(SCRATCH_REGISTER, lhs_lo, rhs.low()));
        self.words.push(encode_sub(hi, hi, SCRATCH_REGISTER));
        Ok(())
    }

    fn lower_wide_comparison(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        relation: &str,
        signed: bool,
    ) -> Result<(), BackendError> {
        let rd = self.dest(instr, op)?;
        let lhs = self.var_location(instr, 0, op)?;
        let rhs = self.var_location(instr, 1, op)?;

        if matches!(relation, "eq" | "ne") {
            self.words.push(encode_xor(rd, lhs.low(), rhs.low()));
            self.copy_or_extend_high(SCRATCH_REGISTER, lhs, signed);
            self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, rhs, signed);
            self.words.push(encode_xor(
                SCRATCH_REGISTER,
                SCRATCH_REGISTER,
                SECOND_SCRATCH_REGISTER,
            ));
            self.words.push(encode_or(rd, rd, SCRATCH_REGISTER));
            self.words.push(match relation {
                "eq" => riscv_encoder::encode_sltiu(rd, rd, 1),
                "ne" => encode_sltu(rd, X0_ZERO, rd),
                _ => unreachable!(),
            });
            return Ok(());
        }

        let different_label = self.internal_label("different");
        let end_label = self.internal_label("end");
        self.copy_or_extend_high(SCRATCH_REGISTER, lhs, signed);
        self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, rhs, signed);
        self.words.push(encode_xor(
            SCRATCH_REGISTER,
            SCRATCH_REGISTER,
            SECOND_SCRATCH_REGISTER,
        ));
        self.record_named_branch(
            different_label.clone(),
            BranchKind::NeZero {
                rs1: SCRATCH_REGISTER,
            },
        );

        self.emit_compare_words(rd, lhs.low(), rhs.low(), relation, false);
        self.record_named_branch(end_label.clone(), BranchKind::Jump);
        self.mark_label(different_label);
        self.copy_or_extend_high(SCRATCH_REGISTER, lhs, signed);
        self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, rhs, signed);
        self.emit_compare_words(
            rd,
            SCRATCH_REGISTER,
            SECOND_SCRATCH_REGISTER,
            relation,
            signed,
        );
        self.mark_label(end_label);
        Ok(())
    }

    fn emit_compare_words(&mut self, rd: u32, lhs: u32, rhs: u32, relation: &str, signed: bool) {
        let less = |lhs, rhs| {
            if signed {
                encode_slt(rd, lhs, rhs)
            } else {
                encode_sltu(rd, lhs, rhs)
            }
        };
        match relation {
            "lt" => self.words.push(less(lhs, rhs)),
            "gt" => self.words.push(less(rhs, lhs)),
            "le" => {
                self.words.push(less(rhs, lhs));
                self.words.push(encode_xori(rd, rd, 1));
            }
            "ge" => {
                self.words.push(less(lhs, rhs));
                self.words.push(encode_xori(rd, rd, 1));
            }
            _ => unreachable!("equality uses the non-branching pair path"),
        }
    }

    fn copy_or_extend_high(&mut self, dest: u32, location: ValueLocation, signed: bool) {
        match location {
            ValueLocation::Pair { hi, .. } => self.words.push(encode_addi(dest, hi, 0)),
            ValueLocation::Word(lo) if signed => self.words.push(encode_srai(dest, lo, 31)),
            ValueLocation::Word(_) => self.words.push(encode_addi(dest, X0_ZERO, 0)),
        }
    }

    fn add_or_extend_high(&mut self, dest: u32, location: ValueLocation, signed: bool) {
        match location {
            ValueLocation::Pair { hi, .. } => self.words.push(encode_add(dest, dest, hi)),
            ValueLocation::Word(lo) if signed => {
                self.words.push(encode_srai(SCRATCH_REGISTER, lo, 31));
                self.words.push(encode_add(dest, dest, SCRATCH_REGISTER));
            }
            ValueLocation::Word(_) => {}
        }
    }

    fn sub_or_extend_high(&mut self, dest: u32, location: ValueLocation, signed: bool) {
        match location {
            ValueLocation::Pair { hi, .. } => self.words.push(encode_sub(dest, dest, hi)),
            ValueLocation::Word(lo) if signed => {
                self.words.push(encode_srai(SCRATCH_REGISTER, lo, 31));
                self.words.push(encode_sub(dest, dest, SCRATCH_REGISTER));
            }
            ValueLocation::Word(_) => {}
        }
    }

    fn load_constant(&mut self, rd: u32, value: u32) {
        let value = value as i32;
        if (-2048..=2047).contains(&value) {
            self.words.push(encode_addi(rd, X0_ZERO, value));
            return;
        }
        let upper = ((value as i64 + 0x800) >> 12) as i32;
        let lower = value as i64 - ((upper as i64) << 12);
        self.words.push(encode_lui(rd, upper as u32));
        if lower != 0 {
            self.words.push(encode_addi(rd, rd, lower as i32));
        }
    }

    fn mask_unsigned(&mut self, rd: u32, ty: &str) {
        let mask = match ty {
            "u4" => Some(0x0f),
            "u8" => Some(0xff),
            "u16" => None,
            _ => None,
        };
        if let Some(mask) = mask {
            self.words.push(encode_andi(rd, rd, mask));
        }
        if ty == "u16" {
            self.load_constant(SCRATCH_REGISTER, 0xffff);
            self.words.push(encode_and(rd, rd, SCRATCH_REGISTER));
        }
    }

    fn require_scalar_type(&self, ty: &str) -> Result<(), BackendError> {
        if is_rv32_value_type(ty) {
            Ok(())
        } else {
            Err(BackendError::UnsupportedType(ty.to_owned()))
        }
    }

    fn require_operation_type(&self, ty: &str) -> Result<(), BackendError> {
        if is_rv32_operation_type(ty) {
            Ok(())
        } else {
            Err(BackendError::UnsupportedType(ty.to_owned()))
        }
    }

    fn require_comparison_type(
        &self,
        instr: &CIRInstr,
        ty: &str,
        op: &str,
    ) -> Result<(), BackendError> {
        if is_rv32_operation_type(ty) {
            return Ok(());
        }
        if !matches!(ty, "i64" | "u64") {
            return Err(BackendError::UnsupportedType(ty.to_owned()));
        }

        for index in 0..2 {
            let name = match instr.srcs.get(index) {
                Some(CIROperand::Var(name)) => name,
                _ => {
                    return Err(BackendError::InvalidOperand(format!(
                        "{op} srcs[{index}] must be Var"
                    )))
                }
            };
            if !self.word_sized_values.contains(name) {
                return Err(BackendError::UnsupportedType(ty.to_owned()));
            }
        }
        Ok(())
    }
}

fn comparison_parts(op: &str) -> Option<(&str, &str)> {
    let rest = op.strip_prefix("cmp_")?;
    for relation in ["eq", "ne", "lt", "le", "gt", "ge"] {
        if let Some(ty) = rest.strip_prefix(&format!("{relation}_")) {
            return Some((relation, ty));
        }
    }
    None
}

fn is_signed(ty: &str) -> bool {
    ty.starts_with('i')
}

fn is_rv32_value_type(ty: &str) -> bool {
    matches!(
        ty,
        "u4" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "bool"
    )
}

fn is_rv32_operation_type(ty: &str) -> bool {
    matches!(
        ty,
        "u4" | "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "bool"
    )
}

fn literal_word(operand: Option<&CIROperand>, ty: &str) -> Result<u32, BackendError> {
    match operand {
        Some(CIROperand::Int(value)) if ty.starts_with('u') => {
            if *value < 0 {
                Ok(
                    i32::try_from(*value).map_err(|_| BackendError::ImmediateOutOfRange(*value))?
                        as u32,
                )
            } else {
                u32::try_from(*value).map_err(|_| BackendError::ImmediateOutOfRange(*value))
            }
        }
        Some(CIROperand::Int(value)) => Ok(i32::try_from(*value)
            .map_err(|_| BackendError::ImmediateOutOfRange(*value))?
            as u32),
        Some(CIROperand::Bool(value)) => Ok(u32::from(*value)),
        _ => Err(BackendError::InvalidOperand(
            "const_* srcs[0] must be Int or Bool".to_owned(),
        )),
    }
}

fn wide_literal_fits_word(operand: Option<&CIROperand>, ty: &str) -> Result<bool, BackendError> {
    let value = match operand {
        Some(CIROperand::Int(value)) => *value,
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_* srcs[0] must be Int".to_owned(),
            ))
        }
    };
    Ok(match ty {
        "i64" => i32::try_from(value).is_ok(),
        "u64" => u32::try_from(value).is_ok(),
        _ => false,
    })
}

fn wide_literal_words(operand: Option<&CIROperand>) -> Result<(u32, u32), BackendError> {
    let value = match operand {
        Some(CIROperand::Int(value)) => *value as u64,
        _ => {
            return Err(BackendError::InvalidOperand(
                "const_i64/const_u64 srcs[0] must be Int".to_owned(),
            ))
        }
    };
    Ok((value as u32, (value >> 32) as u32))
}

fn value_to_rv32(value: &Value) -> Result<u32, BackendError> {
    match value {
        Value::Int(value) => Ok(i32::try_from(*value)
            .map_err(|_| BackendError::ImmediateOutOfRange(*value))?
            as u32),
        Value::Bool(value) => Ok(u32::from(*value)),
        _ => Err(BackendError::InvalidOperand(
            "RV32I runner accepts only integer and boolean arguments".to_owned(),
        )),
    }
}

impl Backend for Riscv32Backend {
    fn name(&self) -> &str {
        "riscv32"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile(
            &FunctionContext {
                name: "<anonymous>",
                params: &[],
                return_type: "void",
            },
            ir,
        )
        .ok()
    }

    fn compile_function(&self, ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile(ctx, ir).ok()
    }

    fn run(&self, binary: &[u8], args: &[Value]) -> Value {
        run_binary(binary, args)
            .map(|result| Value::Int(result.return_value as i64))
            .unwrap_or(Value::Null)
    }
}
