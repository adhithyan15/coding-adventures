//! RV32I backend for the typed CIR stage of the LANG pipeline.
//!
//! This backend deliberately consumes `CIRInstr`, never dynamic IIR.  It is a
//! small but executable scalar lane: supported functions lower to real RV32I
//! bytes, with RV32M multiplication plus unsigned wide division/modulo, and
//! `run_binary` executes those bytes in the in-tree simulator.

use std::collections::{HashMap, HashSet};
use std::fmt;

use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use riscv_encoder::{
    assemble, encode_add, encode_addi, encode_and, encode_andi, encode_beq, encode_bne,
    encode_div, encode_divu, encode_ecall, encode_jal, encode_lui, encode_mul, encode_mulhu,
    encode_lw, encode_or, encode_ori, encode_rem, encode_remu, encode_sll, encode_slli, encode_slt,
    encode_sltu, encode_sra, encode_srai, encode_srl, encode_srli, encode_sub, encode_xor,
    encode_xori, encode_sw, A0, RET_WORD,
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
const DIVISION_TEMP_REGISTER: u32 = 18;
const DIVISION_BORROW_REGISTER: u32 = 19;
const DIVISION_COUNTER_REGISTER: u32 = 20;
const DIVISION_DIVISOR_LOW_REGISTER: u32 = 21;
const DIVISION_DIVISOR_HIGH_REGISTER: u32 = 22;
const DIVISION_LHS_SIGN_REGISTER: u32 = 23;
const DIVISION_QUOTIENT_SIGN_REGISTER: u32 = 24;
const DIVISION_DIVISOR_NONZERO_REGISTER: u32 = 25;
const VALUE_REGISTERS: [u32; 6] = [5, 6, 7, 28, 29, 30];
const VALUE_REGISTER_PAIRS: [(u32, u32); 3] = [(5, 6), (7, 28), (29, 30)];
/// Reserved for scalar results when every temporary pair is live. No current
/// lowering sequence uses `x9`.
const MIXED_WIDTH_REGISTER: u32 = 9;
const COMPARISON_HIGH_REGISTER: u32 = 20;
const STACK_POINTER: u32 = 2;
const SPILLED_LHS_REGISTER: u32 = 26;
const SPILLED_RHS_REGISTER: u32 = 27;

/// A typed CIR function participating in one flat RV32I module image.
///
/// `compile_module` places the selected entry point first, then resolves its
/// direct calls to the remaining function bodies with PC-relative `jal` words.
pub struct ModuleFunction<'a> {
    pub context: FunctionContext<'a>,
    pub cir: &'a [CIRInstr],
}

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
    InFunction {
        function: String,
        error: Box<BackendError>,
    },
    UnsupportedOp(String),
    UnsupportedType(String),
    /// A value the module needs to hold in a register is a floating-point
    /// number, and RV32I — the *base integer* ISA — has no floating-point
    /// registers at all.  See [`is_floating_point_type`] for the reasoning.
    ///
    /// `site` names where the float showed up (`op "const_f64"`, or
    /// `parameter "mag"`) so a caller's message points at real CIR, not just
    /// "somewhere in this function".
    UnsupportedFloat { site: String, ty: String },
    InvalidOperand(String),
    UndefinedVariable(String),
    UndefinedLabel(String),
    ImmediateOutOfRange(i64),
    OutOfRegisters,
    TooManyArguments(usize),
    BranchOutOfRange { label: String, offset: i64 },
    CallOutOfRange { function: String, offset: i64 },
    ExecutionDidNotHalt { steps: usize },
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InFunction { function, error } => {
                write!(f, "riscv-backend: function {function:?}: {error}")
            }
            Self::UnsupportedOp(op) => write!(f, "riscv-backend: unsupported op {op:?}"),
            Self::UnsupportedType(ty) => {
                write!(f, "riscv-backend: unsupported RV32I scalar type {ty:?}")
            }
            Self::UnsupportedFloat { site, ty } => write!(
                f,
                "riscv-backend: {site} carries floating-point type {ty:?}, and RV32I is the \
                 base *integer* ISA — it has no floating-point registers (f32 needs the F \
                 extension, f64 needs D, i.e. RV32F/RV32D).  Retarget this module to a \
                 float-capable backend (LLVM, JVM, CLR, wasm), or lower the float to \
                 soft-float integer sequences before the RV32I backend sees it."
            ),
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
            Self::CallOutOfRange { function, offset } => write!(
                f,
                "riscv-backend: call to function {function:?} has out-of-range offset {offset}"
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
    let mut lowerer = Lowerer::new(ctx, cir, false, None, false)?;
    for instr in cir {
        lowerer.lower(instr)?;
        lowerer.consume_value_sources(instr);
    }
    lowerer.resolve_branches()?;
    if lowerer.words.is_empty() {
        lowerer.words.push(RET_WORD);
    }
    Ok(assemble(&lowerer.words))
}

/// Lower and link a module of CIR functions into one executable RV32I image.
///
/// The entry function begins at address zero so [`run_binary`] can execute the
/// image directly. Direct calls marshal scalar and 64-bit pair values through
/// the starter RV32 ABI. Values that remain live in the caller are deliberately
/// refused until caller-save spilling lands.
pub fn compile_module(
    functions: &[ModuleFunction<'_>],
    entry_point: Option<&str>,
) -> Result<Vec<u8>, BackendError> {
    if functions.is_empty() {
        return Ok(RET_WORD.to_le_bytes().to_vec());
    }
    let mut ordered: Vec<&ModuleFunction<'_>> = functions.iter().collect();
    if let Some(entry_point) = entry_point {
        let entry_index = ordered
            .iter()
            .position(|function| function.context.name == entry_point)
            .ok_or_else(|| BackendError::UndefinedLabel(entry_point.to_owned()))?;
        ordered.swap(0, entry_index);
    }

    let mut function_signatures = HashMap::with_capacity(ordered.len());
    for function in &ordered {
        if function_signatures
            .insert(
                function.context.name.to_owned(),
                FunctionSignature {
                    params: function
                        .context
                        .params
                        .iter()
                        .map(|(_, ty)| ty.clone())
                        .collect(),
                    return_type: function.context.return_type.to_owned(),
                },
            )
            .is_some()
        {
            return Err(BackendError::InvalidOperand(format!(
                "duplicate module function {:?}",
                function.context.name
            )));
        }
    }

    let direct_call_targets: HashSet<String> = ordered
        .iter()
        .flat_map(|function| function.cir)
        .filter(|instr| instr.op == "call")
        .filter_map(|instr| match instr.srcs.first() {
            Some(CIROperand::Var(function)) => Some(function.clone()),
            _ => None,
        })
        .collect();

    let mut lowerers = Vec::with_capacity(ordered.len());
    for function in &ordered {
        let mut lowerer = Lowerer::new(
            &function.context,
            function.cir,
            true,
            Some(&function_signatures),
            direct_call_targets.contains(function.context.name),
        )?;
        for instr in function.cir {
            lowerer.lower(instr).map_err(|error| BackendError::InFunction {
                function: function.context.name.to_owned(),
                error: Box::new(error),
            })?;
            lowerer.consume_value_sources(instr);
        }
        lowerer
            .resolve_branches()
            .map_err(|error| BackendError::InFunction {
                function: function.context.name.to_owned(),
                error: Box::new(error),
            })?;
        if lowerer.words.is_empty() {
            lowerer.words.push(RET_WORD);
        }
        lowerers.push(lowerer);
    }

    let mut function_offsets = HashMap::with_capacity(ordered.len());
    let mut offset = 0usize;
    for (function, lowerer) in ordered.iter().zip(&lowerers) {
        function_offsets.insert(function.context.name.to_owned(), offset);
        offset += lowerer.words.len() * 4;
    }

    let mut bytes = Vec::with_capacity(offset);
    let mut function_offset = 0usize;
    for (function, lowerer) in ordered.iter().zip(&mut lowerers) {
        lowerer
            .resolve_calls(function_offset, &function_offsets)
            .map_err(|error| BackendError::InFunction {
                function: function.context.name.to_owned(),
                error: Box::new(error),
            })?;
        bytes.extend_from_slice(&assemble(&lowerer.words));
        function_offset += lowerer.words.len() * 4;
    }
    if bytes.is_empty() {
        bytes.extend_from_slice(&RET_WORD.to_le_bytes());
    }
    Ok(bytes)
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
    calls: Vec<PendingCall>,
    /// Value uses still to be lowered. The allocator uses this to reclaim dead
    /// scalar values and register pairs before it spills live values.
    remaining_uses: HashMap<String, usize>,
    allow_direct_calls: bool,
    call_signatures: HashMap<String, FunctionSignature>,
    canonicalize_wide_return: bool,
    next_internal_label: usize,
    frame_size: i32,
    return_address_offset: Option<i32>,
    call_argument_words: usize,
    call_save_words: usize,
    next_spill_slot: usize,
}

#[derive(Debug, Clone)]
struct FunctionSignature {
    params: Vec<String>,
    return_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueLocation {
    Word(u32),
    Pair { lo: u32, hi: u32 },
    Spill { offset: i32 },
    PairSpill { lo_offset: i32, hi_offset: i32 },
}

#[derive(Debug, Clone, Copy)]
enum SavedValue {
    Word { register: u32, offset: i32 },
    Pair { lo: u32, hi: u32, offset: i32 },
}

impl ValueLocation {
    fn low(self) -> u32 {
        match self {
            Self::Word(register) | Self::Pair { lo: register, .. } => register,
            Self::Spill { .. } | Self::PairSpill { .. } => {
                unreachable!("wide lowering must materialize a spill slot")
            }
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

#[derive(Debug, Clone)]
struct PendingCall {
    word_index: usize,
    function: String,
}

impl Lowerer {
    fn new(
        ctx: &FunctionContext<'_>,
        cir: &[CIRInstr],
        allow_direct_calls: bool,
        call_signatures: Option<&HashMap<String, FunctionSignature>>,
        canonicalize_wide_return: bool,
    ) -> Result<Self, BackendError> {
        let mut env = Vec::with_capacity(ctx.params.len());
        let mut next_argument = 0;
        for (name, ty) in ctx.params {
            if !is_rv32_value_type(ty) {
                return Err(unsupported_type_error(ty, &format!("parameter {name:?}")));
            }
            let location = if matches!(ty.as_str(), "i64" | "u64") {
                if next_argument + 1 >= ARG_REGISTERS.len() {
                    return Err(BackendError::TooManyArguments(next_argument + 2));
                }
                let pair = ValueLocation::Pair {
                    lo: ARG_REGISTERS[next_argument],
                    hi: ARG_REGISTERS[next_argument + 1],
                };
                next_argument += 2;
                pair
            } else {
                if next_argument >= ARG_REGISTERS.len() {
                    return Err(BackendError::TooManyArguments(next_argument + 1));
                }
                let word = ValueLocation::Word(ARG_REGISTERS[next_argument]);
                next_argument += 1;
                word
            };
            env.push((name.clone(), location));
        }
        let value_word_count: usize = cir
            .iter()
            .filter(|instr| instr.dest.is_some())
            .map(|instr| {
                if matches!(instr.ty.as_str(), "i64" | "u64") {
                    2
                } else {
                    1
                }
            })
            .sum();
        let call_argument_words = match (allow_direct_calls, call_signatures) {
            (true, Some(signatures)) => max_call_argument_words(cir, signatures)?,
            _ => 0,
        };
        let call_save_words = max_call_save_words(ctx, cir);
        let needs_return_address_slot = allow_direct_calls && cir.iter().any(|instr| instr.op == "call");
        let frame_words = value_word_count
            + call_argument_words
            + call_save_words
            + usize::from(needs_return_address_slot);
        let frame_size = if frame_words > VALUE_REGISTERS.len() || needs_return_address_slot {
            ((frame_words as i32) * 4 + 15) & !15
        } else {
            0
        };
        if frame_size > 2048 {
            return Err(BackendError::ImmediateOutOfRange(frame_size as i64));
        }
        let mut words = Vec::new();
        if frame_size != 0 {
            words.push(encode_addi(STACK_POINTER, STACK_POINTER, -frame_size));
        }
        let return_address_offset = needs_return_address_slot.then_some(frame_size - 4);
        if let Some(offset) = return_address_offset {
            words.push(encode_sw(X1_RA, STACK_POINTER, offset));
        }
        Ok(Self {
            words,
            env,
            word_sized_values: HashSet::new(),
            labels: HashMap::new(),
            branches: Vec::new(),
            calls: Vec::new(),
            remaining_uses: count_value_uses(cir),
            allow_direct_calls,
            call_signatures: call_signatures.cloned().unwrap_or_default(),
            canonicalize_wide_return,
            next_internal_label: 0,
            frame_size,
            return_address_offset,
            call_argument_words,
            call_save_words,
            next_spill_slot: call_argument_words + call_save_words,
        })
    }

    fn lower(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        let op = instr.op.as_str();
        if op == "ret_void" {
            self.restore_stack_frame();
            self.words.push(RET_WORD);
            return Ok(());
        }
        if let Some(ty) = op.strip_prefix("ret_") {
            self.require_scalar_type(ty, op)?;
            match self.var_location(instr, 0, op)? {
                ValueLocation::Word(src) => {
                    self.words.push(encode_addi(A0, src, 0));
                    if self.canonicalize_wide_return && matches!(ty, "i64" | "u64") {
                        self.words.push(if is_signed(ty) {
                            encode_srai(A1, src, 31)
                        } else {
                            encode_addi(A1, X0_ZERO, 0)
                        });
                    }
                }
                ValueLocation::Pair { lo, hi } => {
                    self.words.push(encode_addi(A0, lo, 0));
                    self.words.push(encode_addi(A1, hi, 0));
                }
                ValueLocation::Spill { offset } => {
                    self.words.push(encode_lw(A0, STACK_POINTER, offset));
                }
                ValueLocation::PairSpill {
                    lo_offset,
                    hi_offset,
                } => {
                    self.words.push(encode_lw(A0, STACK_POINTER, lo_offset));
                    self.words.push(encode_lw(A1, STACK_POINTER, hi_offset));
                }
            }
            self.restore_stack_frame();
            self.words.push(RET_WORD);
            return Ok(());
        }
        if let Some(ty) = op.strip_prefix("const_") {
            self.require_scalar_type(ty, op)?;
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

        if op == "call" {
            return self.lower_direct_call(instr);
        }

        if let Some(ty) = op.strip_prefix("mov_") {
            self.require_scalar_type(ty, op)?;
            return self.lower_move(instr, ty, op);
        }

        for family in ["add", "sub", "mul", "div", "mod", "and", "or", "xor", "shl", "shr"] {
            if let Some(ty) = op.strip_prefix(&format!("{family}_")) {
                if matches!(ty, "i64" | "u64") {
                    return match family {
                        "add" => self.lower_wide_add(instr, op, is_signed(ty)),
                        "sub" => self.lower_wide_sub(instr, op, is_signed(ty)),
                        "mul" => self.lower_wide_mul(instr, op, is_signed(ty)),
                        "div" | "mod" if !is_signed(ty) => {
                            self.lower_wide_unsigned_divmod(instr, op, family)
                        }
                        "div" | "mod" => self.lower_wide_signed_divmod(instr, op, family),
                        "and" | "or" | "xor" => {
                            self.lower_wide_bitwise(instr, op, family, is_signed(ty))
                        }
                        "shl" => self.lower_wide_shift(instr, op, true, false),
                        "shr" => self.lower_wide_shift(instr, op, false, is_signed(ty)),
                        _ => Err(BackendError::UnsupportedType(ty.to_owned())),
                    };
                }
                self.require_operation_type(ty, op)?;
                let rd = self.dest(instr, op)?;
                let lhs = self.var_src(instr, 0, op)?;
                let rhs = self.var_src(instr, 1, op)?;
                let word = match family {
                    "add" => encode_add(rd, lhs, rhs),
                    "sub" => encode_sub(rd, lhs, rhs),
                    "mul" => encode_mul(rd, lhs, rhs),
                    "div" if is_signed(ty) => encode_div(rd, lhs, rhs),
                    "div" => encode_divu(rd, lhs, rhs),
                    "mod" if is_signed(ty) => encode_rem(rd, lhs, rhs),
                    "mod" => encode_remu(rd, lhs, rhs),
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
                if matches!(ty, "i64" | "u64") {
                    return match family {
                        "not" => self.lower_wide_not(instr, op, is_signed(ty)),
                        _ => Err(BackendError::UnsupportedType(ty.to_owned())),
                    };
                }
                self.require_operation_type(ty, op)?;
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

    fn resolve_calls(
        &mut self,
        function_offset: usize,
        function_offsets: &HashMap<String, usize>,
    ) -> Result<(), BackendError> {
        for call in &self.calls {
            let target = function_offsets
                .get(&call.function)
                .copied()
                .ok_or_else(|| BackendError::UndefinedLabel(call.function.clone()))?;
            let call_site = function_offset + call.word_index * 4;
            let offset = target as i64 - call_site as i64;
            if !(-(1 << 20)..(1 << 20)).contains(&offset) {
                return Err(BackendError::CallOutOfRange {
                    function: call.function.clone(),
                    offset,
                });
            }
            self.words[call.word_index] = encode_jal(X1_RA, offset as i32);
        }
        Ok(())
    }

    fn lower_direct_call(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if !self.allow_direct_calls {
            return Err(BackendError::UnsupportedOp(
                "call (module linking required)".to_owned(),
            ));
        }
        let Some(CIROperand::Var(function)) = instr.srcs.first() else {
            return Err(BackendError::InvalidOperand(
                "call srcs[0] must be Var(function_name)".to_owned(),
            ));
        };
        let signature = self
            .call_signatures
            .get(function)
            .cloned()
            .ok_or_else(|| BackendError::UndefinedLabel(function.clone()))?;
        let arguments = &instr.srcs[1..];
        if arguments.len() != signature.params.len() {
            return Err(BackendError::InvalidOperand(format!(
                "call to {function:?} supplies {} arguments, but its signature requires {}",
                arguments.len(),
                signature.params.len()
            )));
        }
        let argument_words: usize = signature.params.iter().map(|ty| abi_word_count(ty)).sum();
        if argument_words > ARG_REGISTERS.len() {
            return Err(BackendError::TooManyArguments(argument_words));
        }
        let mut argument_word = 0;
        for (index, ty) in signature.params.iter().enumerate() {
            self.stage_call_argument(instr, index + 1, ty, argument_word)?;
            argument_word += abi_word_count(ty);
        }
        for (argument_word, register) in ARG_REGISTERS
            .iter()
            .copied()
            .enumerate()
            .take(argument_words)
        {
            self.words.push(encode_lw(
                register,
                STACK_POINTER,
                (argument_word * 4) as i32,
            ));
        }
        let saved_values = self.save_live_values_across_call(instr);

        self.calls.push(PendingCall {
            word_index: self.words.len(),
            function: function.clone(),
        });
        self.words.push(0);
        self.restore_live_values_after_call(&saved_values);

        let return_type = if instr.ty == "any" {
            signature.return_type
        } else {
            instr.ty.clone()
        };
        match (instr.dest.as_deref(), return_type.as_str()) {
            (None, "void") => Ok(()),
            (Some(_), "void") => Err(BackendError::InvalidOperand(
                "void call must not have a destination".to_owned(),
            )),
            (Some(_), "i64" | "u64") => {
                let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "call")? else {
                    unreachable!("dest_pair always returns a pair")
                };
                self.words.push(encode_addi(lo, A0, 0));
                self.words.push(encode_addi(hi, A1, 0));
                Ok(())
            }
            (Some(_), ty) => {
                self.require_operation_type(ty, "call")?;
                let destination = self.dest(instr, "call")?;
                self.words.push(encode_addi(destination, A0, 0));
                Ok(())
            }
            (None, _) => Err(BackendError::InvalidOperand(
                "non-void call requires a destination".to_owned(),
            )),
        }
    }

    fn lower_move(&mut self, instr: &CIRInstr, ty: &str, op: &str) -> Result<(), BackendError> {
        let source_name = match instr.srcs.first() {
            Some(CIROperand::Var(name)) => name,
            _ => {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} srcs[0] must be Var"
                )))
            }
        };
        if !matches!(ty, "i64" | "u64") {
            let source = self.var_src(instr, 0, op)?;
            let destination = self.dest(instr, op)?;
            self.words.push(encode_addi(destination, source, 0));
            self.mask_unsigned(destination, ty);
            return Ok(());
        }

        let destination_name = instr
            .dest
            .as_deref()
            .ok_or_else(|| BackendError::InvalidOperand(format!("{op} requires a dest")))?;
        let source_is_dead = self.remaining_uses.get(source_name).copied().unwrap_or_default()
            == value_source_occurrences(instr, source_name);
        let source = self.lookup_location(source_name)?;
        if source_is_dead {
            self.env.push((destination_name.to_owned(), source));
            if matches!(source, ValueLocation::Word(_) | ValueLocation::Spill { .. }) {
                self.word_sized_values.insert(destination_name.to_owned());
            }
            return Ok(());
        }

        // A live register pair must first get a stable stack home; its physical
        // pair can then safely become the destination copy.
        if let ValueLocation::Pair { lo, hi } = source {
            self.spill_pair_value(lo, hi);
        }
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        match self.lookup_location(source_name)? {
            ValueLocation::Word(source) => {
                self.words.push(encode_addi(lo, source, 0));
                self.words.push(if is_signed(ty) {
                    encode_srai(hi, source, 31)
                } else {
                    encode_addi(hi, X0_ZERO, 0)
                });
                self.word_sized_values.insert(destination_name.to_owned());
            }
            ValueLocation::Spill { offset } => {
                self.words.push(encode_lw(lo, STACK_POINTER, offset));
                self.words.push(if is_signed(ty) {
                    encode_srai(hi, lo, 31)
                } else {
                    encode_addi(hi, X0_ZERO, 0)
                });
                self.word_sized_values.insert(destination_name.to_owned());
            }
            ValueLocation::PairSpill {
                lo_offset,
                hi_offset,
            } => {
                self.words.push(encode_lw(lo, STACK_POINTER, lo_offset));
                self.words.push(encode_lw(hi, STACK_POINTER, hi_offset));
            }
            ValueLocation::Pair { .. } => unreachable!("live pair was spilled above"),
        }
        Ok(())
    }

    fn save_live_values_across_call(&mut self, instr: &CIRInstr) -> Vec<SavedValue> {
        let mut offset = (self.call_argument_words * 4) as i32;
        let mut saved = Vec::new();
        for (name, location) in &self.env {
            if self.remaining_uses.get(name).copied().unwrap_or_default()
                <= value_source_occurrences(instr, name)
            {
                continue;
            }
            match location {
                ValueLocation::Word(register) => {
                    self.words.push(encode_sw(*register, STACK_POINTER, offset));
                    saved.push(SavedValue::Word {
                        register: *register,
                        offset,
                    });
                    offset += 4;
                }
                ValueLocation::Pair { lo, hi } => {
                    self.words.push(encode_sw(*lo, STACK_POINTER, offset));
                    self.words.push(encode_sw(*hi, STACK_POINTER, offset + 4));
                    saved.push(SavedValue::Pair {
                        lo: *lo,
                        hi: *hi,
                        offset,
                    });
                    offset += 8;
                }
                ValueLocation::Spill { .. } | ValueLocation::PairSpill { .. } => {}
            }
        }
        debug_assert!(
            (offset - (self.call_argument_words * 4) as i32) / 4
                <= self.call_save_words as i32
        );
        saved
    }

    fn restore_live_values_after_call(&mut self, saved: &[SavedValue]) {
        for value in saved {
            match value {
                SavedValue::Word { register, offset } => {
                    self.words.push(encode_lw(*register, STACK_POINTER, *offset));
                }
                SavedValue::Pair { lo, hi, offset } => {
                    self.words.push(encode_lw(*lo, STACK_POINTER, *offset));
                    self.words.push(encode_lw(*hi, STACK_POINTER, *offset + 4));
                }
            }
        }
    }

    fn stage_call_argument(
        &mut self,
        instr: &CIRInstr,
        index: usize,
        ty: &str,
        word_index: usize,
    ) -> Result<(), BackendError> {
        self.require_scalar_type(ty, "call argument")?;
        let offset = (word_index * 4) as i32;
        match (self.var_location(instr, index, "call")?, abi_word_count(ty)) {
            (ValueLocation::Word(register), 1) => {
                self.words.push(encode_sw(register, STACK_POINTER, offset));
            }
            (ValueLocation::Spill { offset: source }, 1) => {
                self.words.push(encode_lw(SCRATCH_REGISTER, STACK_POINTER, source));
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, offset));
            }
            (ValueLocation::Pair { lo, hi }, 2) => {
                self.words.push(encode_sw(lo, STACK_POINTER, offset));
                self.words.push(encode_sw(hi, STACK_POINTER, offset + 4));
            }
            (
                ValueLocation::PairSpill {
                    lo_offset,
                    hi_offset,
                },
                2,
            ) => {
                self.words.push(encode_lw(SCRATCH_REGISTER, STACK_POINTER, lo_offset));
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, offset));
                self.words.push(encode_lw(SCRATCH_REGISTER, STACK_POINTER, hi_offset));
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, offset + 4));
            }
            (ValueLocation::Word(register), 2) => {
                self.words.push(encode_sw(register, STACK_POINTER, offset));
                self.words.push(if is_signed(ty) {
                    encode_srai(SCRATCH_REGISTER, register, 31)
                } else {
                    encode_addi(SCRATCH_REGISTER, X0_ZERO, 0)
                });
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, offset + 4));
            }
            (ValueLocation::Spill { offset: source }, 2) => {
                self.words.push(encode_lw(SCRATCH_REGISTER, STACK_POINTER, source));
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, offset));
                self.words.push(if is_signed(ty) {
                    encode_srai(SCRATCH_REGISTER, SCRATCH_REGISTER, 31)
                } else {
                    encode_addi(SCRATCH_REGISTER, X0_ZERO, 0)
                });
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, offset + 4));
            }
            (location, words) => {
                return Err(BackendError::InvalidOperand(format!(
                    "call argument at srcs[{index}] has location {location:?}, incompatible with {words}-word type {ty:?}"
                )));
            }
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

    /// A right shift reads its low word before writing it and preserves the
    /// source high word in `SECOND_SCRATCH_REGISTER`, so its dead left-hand
    /// pair can safely become the destination.
    fn dest_pair_reusing_dead_lhs(
        &mut self,
        instr: &CIRInstr,
        op: &str,
    ) -> Result<ValueLocation, BackendError> {
        let destination = instr
            .dest
            .as_deref()
            .ok_or_else(|| BackendError::InvalidOperand(format!("{op} requires a dest")))?;
        let Some(CIROperand::Var(source)) = instr.srcs.first() else {
            return Err(BackendError::InvalidOperand(format!(
                "{op} srcs[0] must be Var"
            )));
        };

        if self.remaining_uses.get(source) == Some(&value_source_occurrences(instr, source)) {
            if let ValueLocation::Pair { lo, hi } = self.lookup_location(source)? {
                let location = ValueLocation::Pair { lo, hi };
                self.env.push((destination.to_owned(), location));
                return Ok(location);
            }
        }

        self.allocate_pair(destination)
    }

    fn var_src(&mut self, instr: &CIRInstr, index: usize, op: &str) -> Result<u32, BackendError> {
        let name = match instr.srcs.get(index) {
            Some(CIROperand::Var(name)) => name,
            _ => {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} srcs[{index}] must be Var"
                )))
            }
        };
        match self.lookup_location(name)? {
            ValueLocation::Word(register) => Ok(register),
            // Source frontends such as Nib normalize function signatures to
            // i64, while CIR retains narrow body operations (`add_u8`, etc.).
            // A narrow operation intentionally consumes the low ABI word.
            ValueLocation::Pair { lo, .. } => Ok(lo),
            ValueLocation::Spill { offset } => {
                let register = if index == 0 {
                    SPILLED_LHS_REGISTER
                } else {
                    SPILLED_RHS_REGISTER
                };
                self.words.push(encode_lw(register, STACK_POINTER, offset));
                Ok(register)
            }
            ValueLocation::PairSpill { lo_offset, .. } => {
                let register = if index == 0 {
                    SPILLED_LHS_REGISTER
                } else {
                    SPILLED_RHS_REGISTER
                };
                self.words.push(encode_lw(register, STACK_POINTER, lo_offset));
                Ok(register)
            }
        }
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
                ValueLocation::Spill { .. } => Err(BackendError::InvalidOperand(format!(
                    "{name:?} is already bound in a stack slot"
                ))),
                ValueLocation::PairSpill { .. } => Err(BackendError::InvalidOperand(format!(
                    "{name:?} is already bound in a wide stack slot"
                ))),
            };
        }
        let reg = self.allocate_value_register()?;
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
                ValueLocation::Spill { .. } => Err(BackendError::InvalidOperand(format!(
                    "{name:?} is already bound in a scalar stack slot"
                ))),
                ValueLocation::PairSpill { .. } => Err(BackendError::InvalidOperand(format!(
                    "{name:?} is already bound in a wide stack slot"
                ))),
            };
        }
        let location = self.allocate_pair_registers()?;
        self.env.push((name.to_owned(), location));
        Ok(location)
    }

    fn allocate_value_register(&mut self) -> Result<u32, BackendError> {
        self.release_dead_pair_values();
        self.release_dead_scalar_values();

        if let Some(register) = VALUE_REGISTERS
            .iter()
            .copied()
            .find(|register| !self.register_is_live(*register))
        {
            return Ok(register);
        }

        let index = self.env.iter().position(|(_, location)| {
            matches!(location, ValueLocation::Word(register) if is_scalar_value_register(*register))
        });
        let index = if let Some(index) = index {
            index
        } else if self.env.iter().any(|(_, location)| matches!(location, ValueLocation::Pair { .. })) {
            if !self.register_is_live(MIXED_WIDTH_REGISTER) {
                return Ok(MIXED_WIDTH_REGISTER);
            }
            self.env
                .iter()
                .position(|(_, location)| {
                    matches!(location, ValueLocation::Word(register) if *register == MIXED_WIDTH_REGISTER)
                })
                .expect("live mixed-width register must have an environment entry")
        } else {
            return Err(BackendError::OutOfRegisters);
        };
        let register = match self.env[index].1 {
            ValueLocation::Word(register) => register,
            _ => unreachable!("value-register entry must be a word"),
        };
        let offset = (self.next_spill_slot * 4) as i32;
        self.next_spill_slot += 1;
        self.words.push(encode_sw(register, STACK_POINTER, offset));
        self.env[index].1 = ValueLocation::Spill { offset };
        Ok(register)
    }

    fn allocate_pair_registers(&mut self) -> Result<ValueLocation, BackendError> {
        self.release_dead_pair_values();
        self.release_dead_scalar_values();

        if let Some((lo, hi)) = VALUE_REGISTER_PAIRS.iter().copied().find(|(lo, hi)| {
            !self.pair_has_live_value(*lo, *hi)
                && !self.register_is_live(*lo)
                && !self.register_is_live(*hi)
        }) {
            return Ok(ValueLocation::Pair { lo, hi });
        }

        if let Some((lo, hi)) = VALUE_REGISTER_PAIRS
            .iter()
            .copied()
            .find(|(lo, hi)| !self.pair_has_live_value(*lo, *hi))
        {
            self.spill_scalar_values_in_pair(lo, hi);
            return Ok(ValueLocation::Pair { lo, hi });
        }

        let (lo, hi) = VALUE_REGISTER_PAIRS
            .iter()
            .copied()
            .find(|(lo, hi)| self.pair_has_live_value(*lo, *hi))
            .ok_or(BackendError::OutOfRegisters)?;
        self.spill_pair_value(lo, hi);
        Ok(ValueLocation::Pair { lo, hi })
    }

    fn release_dead_scalar_values(&mut self) {
        self.env.retain(|(name, location)| {
            !matches!(location, ValueLocation::Word(register)
                if is_scalar_value_register(*register)
                    && self.remaining_uses.get(name).copied().unwrap_or_default() == 0)
        });
    }

    fn release_dead_pair_values(&mut self) {
        self.env.retain(|(name, location)| {
            !matches!(location, ValueLocation::Pair { .. }
                if self.remaining_uses.get(name).copied().unwrap_or_default() == 0)
        });
    }

    fn register_is_live(&self, register: u32) -> bool {
        self.env.iter().any(|(_, location)| {
            matches!(location, ValueLocation::Word(current) if *current == register)
                || matches!(location, ValueLocation::Pair { lo, hi }
                    if *lo == register || *hi == register)
        })
    }

    fn pair_has_live_value(&self, lo: u32, hi: u32) -> bool {
        self.env.iter().any(|(_, location)| {
            matches!(location, ValueLocation::Pair { lo: pair_lo, hi: pair_hi }
                if *pair_lo == lo || *pair_lo == hi || *pair_hi == lo || *pair_hi == hi)
        })
    }

    fn spill_scalar_values_in_pair(&mut self, lo: u32, hi: u32) {
        for register in [lo, hi] {
            let Some(index) = self.env.iter().position(|(_, location)| {
                matches!(location, ValueLocation::Word(current) if *current == register)
            }) else {
                continue;
            };
            let offset = (self.next_spill_slot * 4) as i32;
            self.next_spill_slot += 1;
            self.words.push(encode_sw(register, STACK_POINTER, offset));
            self.env[index].1 = ValueLocation::Spill { offset };
        }
    }

    fn spill_pair_value(&mut self, lo: u32, hi: u32) {
        let lo_offset = (self.next_spill_slot * 4) as i32;
        let hi_offset = lo_offset + 4;
        self.next_spill_slot += 2;
        self.words.push(encode_sw(lo, STACK_POINTER, lo_offset));
        self.words.push(encode_sw(hi, STACK_POINTER, hi_offset));
        for (_, location) in &mut self.env {
            if matches!(location, ValueLocation::Pair { lo: pair_lo, hi: pair_hi }
                if *pair_lo == lo && *pair_hi == hi)
            {
                *location = ValueLocation::PairSpill {
                    lo_offset,
                    hi_offset,
                };
            }
        }
    }

    fn wide_var_location(
        &mut self,
        instr: &CIRInstr,
        index: usize,
        op: &str,
    ) -> Result<ValueLocation, BackendError> {
        match self.var_location(instr, index, op)? {
            ValueLocation::PairSpill {
                lo_offset,
                hi_offset,
            } => {
                let (lo, hi) = if index == 0 {
                    (SPILLED_LHS_REGISTER, SPILLED_RHS_REGISTER)
                } else {
                    (DIVISION_TEMP_REGISTER, DIVISION_BORROW_REGISTER)
                };
                self.words.push(encode_lw(lo, STACK_POINTER, lo_offset));
                self.words.push(encode_lw(hi, STACK_POINTER, hi_offset));
                Ok(ValueLocation::Pair { lo, hi })
            }
            ValueLocation::Spill { .. } => Err(BackendError::InvalidOperand(format!(
                "{op} requires a wide value at srcs[{index}]"
            ))),
            location => Ok(location),
        }
    }

    fn wide_divmod_var_location(
        &mut self,
        instr: &CIRInstr,
        index: usize,
        op: &str,
    ) -> Result<ValueLocation, BackendError> {
        match self.var_location(instr, index, op)? {
            ValueLocation::PairSpill {
                lo_offset,
                hi_offset,
            } => {
                let (lo, hi) = if index == 0 {
                    (SPILLED_LHS_REGISTER, SPILLED_RHS_REGISTER)
                } else {
                    (DIVISION_DIVISOR_LOW_REGISTER, DIVISION_DIVISOR_HIGH_REGISTER)
                };
                self.words.push(encode_lw(lo, STACK_POINTER, lo_offset));
                self.words.push(encode_lw(hi, STACK_POINTER, hi_offset));
                Ok(ValueLocation::Pair { lo, hi })
            }
            ValueLocation::Spill { .. } => Err(BackendError::InvalidOperand(format!(
                "{op} requires a wide value at srcs[{index}]"
            ))),
            location => Ok(location),
        }
    }

    fn restore_stack_frame(&mut self) {
        if let Some(offset) = self.return_address_offset {
            self.words.push(encode_lw(X1_RA, STACK_POINTER, offset));
        }
        if self.frame_size != 0 {
            self.words
                .push(encode_addi(STACK_POINTER, STACK_POINTER, self.frame_size));
        }
    }

    fn lookup_location(&self, name: &str) -> Result<ValueLocation, BackendError> {
        self.env
            .iter()
            .find_map(|(existing, location)| (existing == name).then_some(*location))
            .ok_or_else(|| BackendError::UndefinedVariable(name.to_owned()))
    }

    fn consume_value_sources(&mut self, instr: &CIRInstr) {
        for (index, operand) in instr.srcs.iter().enumerate() {
            if !is_value_source(instr, index) {
                continue;
            }
            let CIROperand::Var(name) = operand else {
                continue;
            };
            if let Some(remaining) = self.remaining_uses.get_mut(name) {
                *remaining = remaining.saturating_sub(1);
            }
        }
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
        let lhs = self.wide_var_location(instr, 0, op)?;
        let rhs = self.wide_var_location(instr, 1, op)?;
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
        let lhs = self.wide_var_location(instr, 0, op)?;
        let rhs = self.wide_var_location(instr, 1, op)?;
        let lhs_lo = lhs.low();
        self.words.push(encode_sub(lo, lhs_lo, rhs.low()));
        self.copy_or_extend_high(hi, lhs, signed);
        self.sub_or_extend_high(hi, rhs, signed);
        self.words
            .push(encode_sltu(SCRATCH_REGISTER, lhs_lo, rhs.low()));
        self.words.push(encode_sub(hi, hi, SCRATCH_REGISTER));
        Ok(())
    }

    fn lower_wide_mul(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        signed: bool,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        let lhs = self.wide_var_location(instr, 0, op)?;
        let rhs = self.wide_var_location(instr, 1, op)?;

        // (a_hi * 2^32 + a_lo) * (b_hi * 2^32 + b_lo), modulo 2^64.
        // Only the low word of each cross product contributes to the result.
        self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, lhs, signed);
        self.copy_or_extend_high(SCRATCH_REGISTER, rhs, signed);
        self.words.push(encode_mul(lo, lhs.low(), rhs.low()));
        self.words.push(encode_mulhu(hi, lhs.low(), rhs.low()));
        self.words.push(encode_mul(SCRATCH_REGISTER, lhs.low(), SCRATCH_REGISTER));
        self.words.push(encode_add(hi, hi, SCRATCH_REGISTER));
        self.words.push(encode_mul(SCRATCH_REGISTER, SECOND_SCRATCH_REGISTER, rhs.low()));
        self.words.push(encode_add(hi, hi, SCRATCH_REGISTER));
        Ok(())
    }

    /// Lower unsigned 64-bit division and remainder with the standard
    /// restoring algorithm. The quotient lives in the destination pair while
    /// `x31`/`x27` hold the low/high running remainder, leaving the divisor
    /// untouched.
    /// A zero divisor deliberately needs no special branch: every remainder is
    /// greater than or equal to zero, so the loop produces all-one quotient
    /// bits and preserves the dividend as the remainder, matching RV32M.
    fn lower_wide_unsigned_divmod(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        family: &str,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        let lhs = self.wide_divmod_var_location(instr, 0, op)?;
        let rhs = self.wide_divmod_var_location(instr, 1, op)?;

        self.words.push(encode_addi(lo, lhs.low(), 0));
        self.copy_or_extend_high(hi, lhs, false);
        self.lower_wide_unsigned_divmod_values(ValueLocation::Pair { lo, hi }, rhs, family);
        Ok(())
    }

    fn lower_wide_signed_divmod(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        family: &str,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        let lhs = self.wide_divmod_var_location(instr, 0, op)?;
        let rhs = self.wide_divmod_var_location(instr, 1, op)?;

        self.words.push(encode_addi(lo, lhs.low(), 0));
        self.copy_or_extend_high(hi, lhs, true);
        self.copy_or_extend_high(DIVISION_LHS_SIGN_REGISTER, lhs, true);
        self.words.push(encode_srai(
            DIVISION_LHS_SIGN_REGISTER,
            DIVISION_LHS_SIGN_REGISTER,
            31,
        ));

        self.words
            .push(encode_addi(DIVISION_DIVISOR_LOW_REGISTER, rhs.low(), 0));
        self.copy_or_extend_high(DIVISION_DIVISOR_HIGH_REGISTER, rhs, true);
        self.words.push(encode_srai(
            DIVISION_QUOTIENT_SIGN_REGISTER,
            DIVISION_DIVISOR_HIGH_REGISTER,
            31,
        ));
        self.words.push(encode_xor(
            DIVISION_QUOTIENT_SIGN_REGISTER,
            DIVISION_QUOTIENT_SIGN_REGISTER,
            DIVISION_LHS_SIGN_REGISTER,
        ));
        self.words.push(encode_or(
            DIVISION_TEMP_REGISTER,
            DIVISION_DIVISOR_LOW_REGISTER,
            DIVISION_DIVISOR_HIGH_REGISTER,
        ));
        self.words.push(encode_sltu(
            DIVISION_DIVISOR_NONZERO_REGISTER,
            X0_ZERO,
            DIVISION_TEMP_REGISTER,
        ));

        let lhs_magnitude = self.internal_label("sdiv_lhs_magnitude");
        self.record_named_branch(
            lhs_magnitude.clone(),
            BranchKind::EqZero {
                rs1: DIVISION_LHS_SIGN_REGISTER,
            },
        );
        self.negate_pair(lo, hi);
        self.mark_label(lhs_magnitude);

        let rhs_magnitude = self.internal_label("sdiv_rhs_magnitude");
        // Preserve the quotient sign above, then reconstruct the divisor sign
        // from its original high word before normalization.
        self.words.push(encode_srai(
            DIVISION_TEMP_REGISTER,
            DIVISION_DIVISOR_HIGH_REGISTER,
            31,
        ));
        self.record_named_branch(
            rhs_magnitude.clone(),
            BranchKind::EqZero {
                rs1: DIVISION_TEMP_REGISTER,
            },
        );
        self.negate_pair(
            DIVISION_DIVISOR_LOW_REGISTER,
            DIVISION_DIVISOR_HIGH_REGISTER,
        );
        self.mark_label(rhs_magnitude);

        self.lower_wide_unsigned_divmod_values(
            ValueLocation::Pair { lo, hi },
            ValueLocation::Pair {
                lo: DIVISION_DIVISOR_LOW_REGISTER,
                hi: DIVISION_DIVISOR_HIGH_REGISTER,
            },
            family,
        );

        let sign_done = self.internal_label("sdiv_sign_done");
        if family == "div" {
            self.record_named_branch(
                sign_done.clone(),
                BranchKind::EqZero {
                    rs1: DIVISION_DIVISOR_NONZERO_REGISTER,
                },
            );
            self.record_named_branch(
                sign_done.clone(),
                BranchKind::EqZero {
                    rs1: DIVISION_QUOTIENT_SIGN_REGISTER,
                },
            );
        } else {
            self.record_named_branch(
                sign_done.clone(),
                BranchKind::EqZero {
                    rs1: DIVISION_LHS_SIGN_REGISTER,
                },
            );
        }
        self.negate_pair(lo, hi);
        self.mark_label(sign_done);
        Ok(())
    }

    fn lower_wide_unsigned_divmod_values(
        &mut self,
        destination: ValueLocation,
        rhs: ValueLocation,
        family: &str,
    ) {
        let ValueLocation::Pair { lo, hi } = destination else {
            unreachable!("wide division always has a pair destination")
        };
        let rhs_hi = match rhs {
            ValueLocation::Pair { hi, .. } => hi,
            ValueLocation::Word(_) => X0_ZERO,
            ValueLocation::Spill { .. } => {
                unreachable!("wide division cannot use a scalar spill slot")
            }
            ValueLocation::PairSpill { .. } => {
                unreachable!("wide division must materialize a pair spill slot")
            }
        };

        self.words.push(encode_addi(SCRATCH_REGISTER, X0_ZERO, 0));
        self.words
            .push(encode_addi(SECOND_SCRATCH_REGISTER, X0_ZERO, 0));
        self.words
            .push(encode_addi(DIVISION_COUNTER_REGISTER, X0_ZERO, 64));

        let loop_label = self.internal_label("udiv_loop");
        let subtract_label = self.internal_label("udiv_subtract");
        let next_label = self.internal_label("udiv_next");
        self.mark_label(loop_label.clone());

        // Shift the next quotient bit into the remainder, then left-shift
        // the quotient so its low bit can record whether subtraction occurs.
        self.words
            .push(encode_srli(DIVISION_TEMP_REGISTER, hi, 31));
        self.words.push(encode_srli(
            DIVISION_BORROW_REGISTER,
            SCRATCH_REGISTER,
            31,
        ));
        self.words.push(encode_slli(
            SECOND_SCRATCH_REGISTER,
            SECOND_SCRATCH_REGISTER,
            1,
        ));
        self.words.push(encode_or(
            SECOND_SCRATCH_REGISTER,
            SECOND_SCRATCH_REGISTER,
            DIVISION_BORROW_REGISTER,
        ));
        self.words
            .push(encode_slli(SCRATCH_REGISTER, SCRATCH_REGISTER, 1));
        self.words.push(encode_or(
            SCRATCH_REGISTER,
            SCRATCH_REGISTER,
            DIVISION_TEMP_REGISTER,
        ));
        self.words
            .push(encode_srli(DIVISION_TEMP_REGISTER, lo, 31));
        self.words.push(encode_slli(hi, hi, 1));
        self.words
            .push(encode_or(hi, hi, DIVISION_TEMP_REGISTER));
        self.words.push(encode_slli(lo, lo, 1));

        // Compare the two-word remainder with the divisor without needing a
        // branch kind beyond "nonzero". A high-word difference decides first;
        // equal highs fall through to the unsigned low-word comparison.
        self.words.push(encode_sltu(
            DIVISION_TEMP_REGISTER,
            SECOND_SCRATCH_REGISTER,
            rhs_hi,
        ));
        self.record_named_branch(
            next_label.clone(),
            BranchKind::NeZero {
                rs1: DIVISION_TEMP_REGISTER,
            },
        );
        self.words.push(encode_sltu(
            DIVISION_TEMP_REGISTER,
            rhs_hi,
            SECOND_SCRATCH_REGISTER,
        ));
        self.record_named_branch(
            subtract_label.clone(),
            BranchKind::NeZero {
                rs1: DIVISION_TEMP_REGISTER,
            },
        );
        self.words.push(encode_sltu(
            DIVISION_TEMP_REGISTER,
            SCRATCH_REGISTER,
            rhs.low(),
        ));
        self.record_named_branch(
            next_label.clone(),
            BranchKind::NeZero {
                rs1: DIVISION_TEMP_REGISTER,
            },
        );

        self.mark_label(subtract_label);
        self.words.push(encode_sltu(
            DIVISION_BORROW_REGISTER,
            SCRATCH_REGISTER,
            rhs.low(),
        ));
        self.words
            .push(encode_sub(SCRATCH_REGISTER, SCRATCH_REGISTER, rhs.low()));
        self.words.push(encode_sub(
            SECOND_SCRATCH_REGISTER,
            SECOND_SCRATCH_REGISTER,
            rhs_hi,
        ));
        self.words.push(encode_sub(
            SECOND_SCRATCH_REGISTER,
            SECOND_SCRATCH_REGISTER,
            DIVISION_BORROW_REGISTER,
        ));
        self.words.push(encode_ori(lo, lo, 1));

        self.mark_label(next_label);
        self.words.push(encode_addi(
            DIVISION_COUNTER_REGISTER,
            DIVISION_COUNTER_REGISTER,
            -1,
        ));
        self.record_named_branch(
            loop_label,
            BranchKind::NeZero {
                rs1: DIVISION_COUNTER_REGISTER,
            },
        );

        if family == "mod" {
            self.words.push(encode_addi(lo, SCRATCH_REGISTER, 0));
            self.words
                .push(encode_addi(hi, SECOND_SCRATCH_REGISTER, 0));
        }
    }

    fn negate_pair(&mut self, lo: u32, hi: u32) {
        self.words.push(encode_sub(lo, X0_ZERO, lo));
        self.words
            .push(encode_sltu(DIVISION_TEMP_REGISTER, X0_ZERO, lo));
        self.words.push(encode_sub(hi, X0_ZERO, hi));
        self.words
            .push(encode_sub(hi, hi, DIVISION_TEMP_REGISTER));
    }

    fn lower_wide_bitwise(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        family: &str,
        signed: bool,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        let lhs = self.wide_var_location(instr, 0, op)?;
        let rhs = self.wide_var_location(instr, 1, op)?;
        let encode = |rd, left, right| match family {
            "and" => encode_and(rd, left, right),
            "or" => encode_or(rd, left, right),
            "xor" => encode_xor(rd, left, right),
            _ => unreachable!("only bitwise families use this helper"),
        };
        self.words.push(encode(lo, lhs.low(), rhs.low()));
        self.copy_or_extend_high(SCRATCH_REGISTER, lhs, signed);
        self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, rhs, signed);
        self.words
            .push(encode(hi, SCRATCH_REGISTER, SECOND_SCRATCH_REGISTER));
        Ok(())
    }

    fn lower_wide_not(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        signed: bool,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
            unreachable!("dest_pair always returns a pair")
        };
        let src = self.wide_var_location(instr, 0, op)?;
        self.words.push(encode_xori(lo, src.low(), -1));
        self.copy_or_extend_high(SCRATCH_REGISTER, src, signed);
        self.words.push(encode_xori(hi, SCRATCH_REGISTER, -1));
        Ok(())
    }

    fn lower_wide_shift(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        left: bool,
        arithmetic_right: bool,
    ) -> Result<(), BackendError> {
        let destination = if left {
            self.dest_pair(instr, op)?
        } else {
            self.dest_pair_reusing_dead_lhs(instr, op)?
        };
        let ValueLocation::Pair { lo, hi } = destination else {
            unreachable!("dest_pair always returns a pair")
        };
        let value = self.wide_var_location(instr, 0, op)?;
        let count = match self.var_location(instr, 1, op)? {
            ValueLocation::Word(register) => register,
            ValueLocation::Pair { .. } => {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires a shift count that fits in one RV32 register"
                )))
            }
            ValueLocation::Spill { .. } => {
                return Err(BackendError::UnsupportedType(
                    "spilled wide shift count".to_owned(),
                ))
            }
            ValueLocation::PairSpill { .. } => {
                return Err(BackendError::InvalidOperand(format!(
                    "{op} requires a shift count that fits in one RV32 register"
                )))
            }
        };
        let signed_value = arithmetic_right;
        self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, value, signed_value);

        let in_range_label = self.internal_label("shift_in_range");
        let under_word_label = self.internal_label("shift_under_word");
        let zero_label = self.internal_label("shift_zero");
        let end_label = self.internal_label("shift_end");

        self.words
            .push(riscv_encoder::encode_sltiu(SCRATCH_REGISTER, count, 64));
        self.record_named_branch(
            in_range_label.clone(),
            BranchKind::NeZero {
                rs1: SCRATCH_REGISTER,
            },
        );
        if arithmetic_right {
            self.words
                .push(encode_srai(lo, SECOND_SCRATCH_REGISTER, 31));
            self.words
                .push(encode_srai(hi, SECOND_SCRATCH_REGISTER, 31));
        } else {
            self.words.push(encode_addi(lo, X0_ZERO, 0));
            self.words.push(encode_addi(hi, X0_ZERO, 0));
        }
        self.record_named_branch(end_label.clone(), BranchKind::Jump);

        self.mark_label(in_range_label);
        self.words
            .push(riscv_encoder::encode_sltiu(SCRATCH_REGISTER, count, 32));
        self.record_named_branch(
            under_word_label.clone(),
            BranchKind::NeZero {
                rs1: SCRATCH_REGISTER,
            },
        );
        self.words.push(encode_addi(SCRATCH_REGISTER, count, -32));
        if left {
            self.words
                .push(encode_sll(hi, value.low(), SCRATCH_REGISTER));
            self.words.push(encode_addi(lo, X0_ZERO, 0));
        } else {
            let shift = if arithmetic_right {
                encode_sra
            } else {
                encode_srl
            };
            self.words
                .push(shift(lo, SECOND_SCRATCH_REGISTER, SCRATCH_REGISTER));
            if arithmetic_right {
                self.words
                    .push(encode_srai(hi, SECOND_SCRATCH_REGISTER, 31));
            } else {
                self.words.push(encode_addi(hi, X0_ZERO, 0));
            }
        }
        self.record_named_branch(end_label.clone(), BranchKind::Jump);

        self.mark_label(under_word_label);
        self.record_named_branch(zero_label.clone(), BranchKind::EqZero { rs1: count });
        if left {
            self.words.push(encode_sll(lo, value.low(), count));
            self.words
                .push(encode_sll(hi, SECOND_SCRATCH_REGISTER, count));
            self.words
                .push(encode_sub(SCRATCH_REGISTER, X0_ZERO, count));
            self.words
                .push(encode_addi(SCRATCH_REGISTER, SCRATCH_REGISTER, 32));
            self.words
                .push(encode_srl(SCRATCH_REGISTER, value.low(), SCRATCH_REGISTER));
            self.words.push(encode_or(hi, hi, SCRATCH_REGISTER));
        } else {
            let shift = if arithmetic_right {
                encode_sra
            } else {
                encode_srl
            };
            self.words.push(shift(lo, value.low(), count));
            self.words
                .push(encode_sub(SCRATCH_REGISTER, X0_ZERO, count));
            self.words
                .push(encode_addi(SCRATCH_REGISTER, SCRATCH_REGISTER, 32));
            self.words.push(encode_sll(
                SCRATCH_REGISTER,
                SECOND_SCRATCH_REGISTER,
                SCRATCH_REGISTER,
            ));
            self.words.push(encode_or(lo, lo, SCRATCH_REGISTER));
            self.words.push(shift(hi, SECOND_SCRATCH_REGISTER, count));
        }
        self.record_named_branch(end_label.clone(), BranchKind::Jump);

        self.mark_label(zero_label);
        self.words.push(encode_addi(lo, value.low(), 0));
        self.words.push(encode_addi(hi, SECOND_SCRATCH_REGISTER, 0));
        self.mark_label(end_label);
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
        let lhs = self.wide_var_location(instr, 0, op)?;
        let rhs = self.wide_var_location(instr, 1, op)?;

        if matches!(relation, "eq" | "ne") {
            self.words.push(encode_xor(rd, lhs.low(), rhs.low()));
            self.copy_or_extend_high(SCRATCH_REGISTER, lhs, signed);
            self.copy_or_extend_high(COMPARISON_HIGH_REGISTER, rhs, signed);
            self.words.push(encode_xor(
                SCRATCH_REGISTER,
                SCRATCH_REGISTER,
                COMPARISON_HIGH_REGISTER,
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
        self.copy_or_extend_high(COMPARISON_HIGH_REGISTER, rhs, signed);
        self.words.push(encode_xor(
            rd,
            SCRATCH_REGISTER,
            COMPARISON_HIGH_REGISTER,
        ));
        self.record_named_branch(
            different_label.clone(),
            BranchKind::NeZero {
                rs1: rd,
            },
        );

        self.emit_compare_words(rd, lhs.low(), rhs.low(), relation, false);
        self.record_named_branch(end_label.clone(), BranchKind::Jump);
        self.mark_label(different_label);
        self.emit_compare_words(
            rd,
            SCRATCH_REGISTER,
            COMPARISON_HIGH_REGISTER,
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
            ValueLocation::Spill { .. } => {
                unreachable!("wide arithmetic cannot use a scalar spill slot")
            }
            ValueLocation::PairSpill { .. } => {
                unreachable!("wide arithmetic must materialize a pair spill slot")
            }
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
            ValueLocation::Spill { .. } => {
                unreachable!("wide arithmetic cannot use a scalar spill slot")
            }
            ValueLocation::PairSpill { .. } => {
                unreachable!("wide arithmetic must materialize a pair spill slot")
            }
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
            ValueLocation::Spill { .. } => {
                unreachable!("wide arithmetic cannot use a scalar spill slot")
            }
            ValueLocation::PairSpill { .. } => {
                unreachable!("wide arithmetic must materialize a pair spill slot")
            }
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

    fn require_scalar_type(&self, ty: &str, op: &str) -> Result<(), BackendError> {
        if is_rv32_value_type(ty) {
            Ok(())
        } else {
            Err(unsupported_type_error(ty, &format!("op {op:?}")))
        }
    }

    fn require_operation_type(&self, ty: &str, op: &str) -> Result<(), BackendError> {
        if is_rv32_operation_type(ty) {
            Ok(())
        } else {
            Err(unsupported_type_error(ty, &format!("op {op:?}")))
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
            return Err(unsupported_type_error(ty, &format!("op {op:?}")));
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

fn count_value_uses(cir: &[CIRInstr]) -> HashMap<String, usize> {
    let mut uses = HashMap::new();
    for instr in cir {
        for (index, operand) in instr.srcs.iter().enumerate() {
            if !is_value_source(instr, index) {
                continue;
            }
            if let CIROperand::Var(name) = operand {
                *uses.entry(name.clone()).or_insert(0) += 1;
            }
        }
    }
    uses
}

fn abi_word_count(ty: &str) -> usize {
    usize::from(matches!(ty, "i64" | "u64")) + 1
}

fn max_call_argument_words(
    cir: &[CIRInstr],
    signatures: &HashMap<String, FunctionSignature>,
) -> Result<usize, BackendError> {
    let mut maximum = 0;
    for instr in cir.iter().filter(|instr| instr.op == "call") {
        let Some(CIROperand::Var(function)) = instr.srcs.first() else {
            return Err(BackendError::InvalidOperand(
                "call srcs[0] must be Var(function_name)".to_owned(),
            ));
        };
        let signature = signatures
            .get(function)
            .ok_or_else(|| BackendError::UndefinedLabel(function.clone()))?;
        let words: usize = signature.params.iter().map(|ty| abi_word_count(ty)).sum();
        if words > ARG_REGISTERS.len() {
            return Err(BackendError::TooManyArguments(words));
        }
        maximum = maximum.max(words);
    }
    Ok(maximum)
}

fn max_call_save_words(ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> usize {
    let mut value_types: HashMap<&str, &str> = ctx
        .params
        .iter()
        .map(|(name, ty)| (name.as_str(), ty.as_str()))
        .collect();
    for instr in cir {
        if let Some(destination) = instr.dest.as_deref() {
            value_types.insert(destination, instr.ty.as_str());
        }
    }

    let mut maximum = 0;
    for (call_index, call) in cir.iter().enumerate() {
        if call.op != "call" {
            continue;
        }
        let mut live_values = HashSet::new();
        for instr in &cir[call_index + 1..] {
            for (index, operand) in instr.srcs.iter().enumerate() {
                if !is_value_source(instr, index) {
                    continue;
                }
                let CIROperand::Var(name) = operand else {
                    continue;
                };
                if value_types.contains_key(name.as_str()) {
                    live_values.insert(name.as_str());
                }
            }
        }
        maximum = maximum.max(
            live_values
                .into_iter()
                .map(|name| storage_word_count(value_types[name]))
                .sum(),
        );
    }
    maximum
}

fn storage_word_count(ty: &str) -> usize {
    if matches!(ty, "i64" | "u64" | "any") {
        2
    } else {
        1
    }
}

fn value_source_occurrences(instr: &CIRInstr, name: &str) -> usize {
    instr
        .srcs
        .iter()
        .enumerate()
        .filter(|(index, operand)| {
            is_value_source(instr, *index)
                && matches!(operand, CIROperand::Var(candidate) if candidate == name)
        })
        .count()
}

fn is_value_source(instr: &CIRInstr, index: usize) -> bool {
    match instr.op.as_str() {
        "label" | "jmp" => false,
        "call" => index != 0,
        "jmp_if_false" | "br_false_bool" | "jmp_if_true" | "br_true_bool" => index == 0,
        _ => true,
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

/// Is `ty` a floating-point CIR scalar type?
///
/// `aot-core`'s inference spells every literal `Float` as `"f64"` and its
/// type lattice also knows `"f32"`, so those are the two names that can reach
/// a backend today; the wider IEEE-754 names are listed for the day the
/// lattice grows them.
///
/// # Why this deserves its own error
///
/// RV32I is the RISC-V **base integer** ISA.  Its entire architectural state
/// is 32 general-purpose *integer* registers — there is no `f0`..`f31` bank,
/// no `fadd.d`, no way to even name a double.  Floating point is a separate,
/// optional standard extension: `F` (single precision, adds RV32F) and `D`
/// (double precision, adds RV32D).  So an `f64` on RV32I is not "an op we
/// have not written yet" — it is a value the target cannot represent at all
/// until either the module is retargeted or the float is decomposed into
/// integer soft-float sequences.  Saying that plainly beats a generic
/// "unsupported type", because the two have completely different fixes.
fn is_floating_point_type(ty: &str) -> bool {
    matches!(ty, "f16" | "f32" | "f64" | "f128")
}

/// Build the right "this type does not fit RV32I" error for `ty`.
///
/// Floats get the specific [`BackendError::UnsupportedFloat`] refusal (the
/// fix is retarget-or-soft-float); everything else stays the generic
/// [`BackendError::UnsupportedType`] (the fix is "implement the lowering").
fn unsupported_type_error(ty: &str, site: &str) -> BackendError {
    if is_floating_point_type(ty) {
        BackendError::UnsupportedFloat {
            site: site.to_owned(),
            ty: ty.to_owned(),
        }
    } else {
        BackendError::UnsupportedType(ty.to_owned())
    }
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

fn is_scalar_value_register(register: u32) -> bool {
    VALUE_REGISTERS.contains(&register) || register == MIXED_WIDTH_REGISTER
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
