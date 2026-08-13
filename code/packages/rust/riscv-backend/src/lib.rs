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
    encode_sra, encode_srl, encode_sub, encode_xor, encode_xori, A0, RET_WORD, X0_ZERO, X1_RA,
};
use riscv_simulator::RiscVSimulator;
use vm_core::value::Value;

const DEFAULT_MEMORY_SIZE: usize = 64 * 1024;
const DEFAULT_STEP_LIMIT: usize = 100_000;
const ARG_REGISTERS: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
const SCRATCH_REGISTER: u32 = 31;
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
        halted: result.halted,
        steps: result.steps,
    })
}

struct Lowerer {
    words: Vec<u32>,
    env: Vec<(String, u32)>,
    /// Values known to fit in one RV32 register despite an `i64`/`u64` CIR type.
    word_sized_values: HashSet<String>,
    labels: HashMap<String, usize>,
    branches: Vec<PendingBranch>,
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
        for (index, (name, ty)) in ctx.params.iter().enumerate() {
            if !is_rv32_operation_type(ty) {
                return Err(BackendError::UnsupportedType(ty.clone()));
            }
            env.push((name.clone(), ARG_REGISTERS[index]));
        }
        Ok(Self {
            words: Vec::new(),
            env,
            word_sized_values: HashSet::new(),
            labels: HashMap::new(),
            branches: Vec::new(),
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
            let src = self.var_src(instr, 0, op)?;
            self.words.push(encode_addi(A0, src, 0));
            self.words.push(RET_WORD);
            return Ok(());
        }
        if let Some(ty) = op.strip_prefix("const_") {
            self.require_scalar_type(ty)?;
            let rd = self.dest(instr, op)?;
            self.load_constant(rd, literal_word(instr.srcs.first(), ty)?);
            self.mask_unsigned(rd, ty);
            if matches!(ty, "i64" | "u64") {
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

    fn dest(&mut self, instr: &CIRInstr, op: &str) -> Result<u32, BackendError> {
        let name = instr
            .dest
            .as_deref()
            .ok_or_else(|| BackendError::InvalidOperand(format!("{op} requires a dest")))?;
        self.allocate(name)
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

    fn label_src(&self, instr: &CIRInstr, index: usize, op: &str) -> Result<String, BackendError> {
        match instr.srcs.get(index) {
            Some(CIROperand::Var(name)) => Ok(name.clone()),
            _ => Err(BackendError::InvalidOperand(format!(
                "{op} srcs[{index}] must be a label Var"
            ))),
        }
    }

    fn allocate(&mut self, name: &str) -> Result<u32, BackendError> {
        if let Some((_, reg)) = self.env.iter().find(|(existing, _)| existing == name) {
            return Ok(*reg);
        }
        if self.env.len() >= VALUE_REGISTERS.len() {
            return Err(BackendError::OutOfRegisters);
        }
        let reg = VALUE_REGISTERS[self.env.len()];
        self.env.push((name.to_owned(), reg));
        Ok(reg)
    }

    fn lookup(&self, name: &str) -> Result<u32, BackendError> {
        self.env
            .iter()
            .find_map(|(existing, reg)| (existing == name).then_some(*reg))
            .ok_or_else(|| BackendError::UndefinedVariable(name.to_owned()))
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
