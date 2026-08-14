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
    encode_lbu, encode_lw, encode_or, encode_ori, encode_rem, encode_remu, encode_sb, encode_sll,
    encode_slli, encode_slt,
    encode_sltu, encode_sra, encode_srai, encode_srl, encode_srli, encode_sub, encode_xor,
    encode_xori, encode_sw, A0, RET_WORD,
    X0_ZERO, X1_RA,
};
use riscv_simulator::{
    HostEvent, RiscVSimulator, HOST_ECALL_EXIT, HOST_ECALL_F64_ADD, HOST_ECALL_F64_CMP,
    HOST_ECALL_F64_DIV, HOST_ECALL_F64_FLOOR, HOST_ECALL_F64_MUL, HOST_ECALL_F64_SUB,
    HOST_ECALL_F64_TO_I64_TRUNC, HOST_ECALL_I64_TO_F64, HOST_ECALL_READ_BYTE,
    HOST_ECALL_SERVICE_REGISTER, HOST_ECALL_WRITE_BYTE, HOST_ECALL_WRITE_F64,
    HOST_ECALL_WRITE_I64,
};
use vm_core::value::Value;

const DEFAULT_MEMORY_SIZE: usize = 64 * 1024;
const DEFAULT_STEP_LIMIT: usize = 100_000;
const MEMORY_BOUNDS_EXIT_CODE: i32 = 2;
const ARG_REGISTERS: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];
const A1: u32 = 11;
const A2: u32 = 12;
const A3: u32 = 13;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub return_value: i32,
    pub return_value_high: u32,
    pub halted: bool,
    pub steps: usize,
    /// Signed integer values written through the simulator host ABI.
    pub output: Vec<i64>,
    /// Bytes written through the simulator character-output service.
    pub byte_output: Vec<u8>,
    /// Guest status supplied to the host exit service, if one was used.
    pub exit_code: Option<i32>,
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
    /// A floating-point form is not supported by the RV32I lowering.
    ///
    /// `f64` transport values are the exception: they use an opaque low/high
    /// integer pair, and supported arithmetic, comparisons, and signed i64
    /// conversions use simulator host services. `f32` has no transport
    /// representation yet. See
    /// [`is_floating_point_type`] for the target-level reasoning.
    ///
    /// `site` names where the float showed up (`op "add_f64"`, or
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
                "riscv-backend: {site} carries unsupported floating-point type {ty:?}, and \
                 RV32I is the base *integer* ISA — it has no floating-point registers (f32 \
                 needs the F extension, f64 needs D, i.e. RV32F/RV32D). Supported f64 values \
                 use a simulator soft-float integer-pair ABI; add the missing lowering or \
                 retarget this module to a float-capable backend (LLVM, JVM, CLR, wasm)."
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
    let mut lowerer = Lowerer::new(ctx, cir, false, None, None, None, false)?;
    for instr in cir {
        lowerer.lower(instr)?;
        lowerer.store_f64_home(instr)?;
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
    let global_layout = GlobalLayout::collect(&ordered)?;
    let allocation_layout = ByteAllocationLayout::collect(&ordered, global_layout.byte_len)?;

    let mut lowerers = Vec::with_capacity(ordered.len());
    for function in &ordered {
        let mut lowerer = Lowerer::new(
            &function.context,
            function.cir,
            true,
            Some(&function_signatures),
            Some(&global_layout),
            Some(&allocation_layout),
            direct_call_targets.contains(function.context.name),
        )?;
        for instr in function.cir {
            lowerer.lower(instr).map_err(|error| BackendError::InFunction {
                function: function.context.name.to_owned(),
                error: Box::new(error),
            })?;
            lowerer.store_f64_home(instr).map_err(|error| BackendError::InFunction {
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
        lowerer
            .resolve_data_addresses(offset)
            .map_err(|error| BackendError::InFunction {
                function: function.context.name.to_owned(),
                error: Box::new(error),
            })?;
        bytes.extend_from_slice(&assemble(&lowerer.words));
        function_offset += lowerer.words.len() * 4;
    }
    bytes.resize(bytes.len() + allocation_layout.byte_len, 0);
    for image in allocation_layout.images.values() {
        let start = offset + image.offset;
        bytes[start..start + image.bytes.len()].copy_from_slice(&image.bytes);
    }
    if let Some(cursor_offset) = allocation_layout.heap_cursor_offset {
        let heap_start = offset
            .checked_add(allocation_layout.byte_len)
            .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))?;
        let heap_start = u32::try_from(heap_start)
            .map_err(|_| BackendError::ImmediateOutOfRange(heap_start as i64))?;
        bytes[offset + cursor_offset..offset + cursor_offset + 4]
            .copy_from_slice(&heap_start.to_le_bytes());
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
    run_binary_with_input(binary, args, &[])
}

/// Run a function binary with bytes supplied to the simulator character-input service.
pub fn run_binary_with_input(
    binary: &[u8],
    args: &[Value],
    input: &[u8],
) -> Result<RunResult, BackendError> {
    if args.len() > ARG_REGISTERS.len() {
        return Err(BackendError::TooManyArguments(args.len()));
    }

    let mut program = binary.to_vec();
    program.resize((program.len() + 3) & !3, 0);
    let return_trampoline = program.len();
    // A program may have used a host service immediately before returning.
    // Clear a7 so the runner's terminal ecall keeps its historical halt-only
    // meaning rather than replaying that last service.
    program.extend_from_slice(
        &encode_addi(HOST_ECALL_SERVICE_REGISTER as u32, X0_ZERO, 0).to_le_bytes(),
    );
    program.extend_from_slice(&encode_ecall().to_le_bytes());

    let mut simulator = RiscVSimulator::new(DEFAULT_MEMORY_SIZE);
    simulator.set_host_input(input);
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
        output: simulator
            .host_events
            .iter()
            .filter_map(|event| match event {
                HostEvent::WriteI64(value) => Some(*value),
                HostEvent::WriteByte(_) | HostEvent::ReadByte(_) | HostEvent::Exit(_) => None,
            })
            .collect(),
        byte_output: simulator
            .host_events
            .iter()
            .filter_map(|event| match event {
                HostEvent::WriteByte(value) => Some(*value),
                HostEvent::WriteI64(_) | HostEvent::ReadByte(_) | HostEvent::Exit(_) => None,
            })
            .collect(),
        exit_code: simulator.exit_code,
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
    global_layout: Option<GlobalLayout>,
    allocation_layout: Option<ByteAllocationLayout>,
    function_name: String,
    pending_globals: Vec<PendingGlobal>,
    /// The old location of a non-SSA destination while its defining
    /// instruction is still reading it as a source.
    pending_reassignment: Option<PendingReassignment>,
    /// Value uses still to be lowered. The allocator uses this to reclaim dead
    /// scalar values and register pairs before it spills live values.
    remaining_uses: HashMap<String, usize>,
    /// Values read by a backward branch's loop body stay allocated across the
    /// static end of that body, because the emitted code can execute it again.
    loop_carried_values: HashSet<String>,
    allow_direct_calls: bool,
    call_signatures: HashMap<String, FunctionSignature>,
    canonicalize_wide_return: bool,
    next_internal_label: usize,
    frame_size: i32,
    return_address_offset: Option<i32>,
    call_argument_words: usize,
    call_save_words: usize,
    /// f64 values, plus pair values in a function with a backward branch, have
    /// stable frame homes. This makes values available at control-flow joins
    /// and on later dynamic loop iterations.
    f64_homes: HashMap<String, ValueLocation>,
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

#[derive(Debug, Clone)]
struct PendingReassignment {
    name: String,
    old_location: ValueLocation,
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

#[derive(Debug, Clone)]
struct PendingGlobal {
    word_index: usize,
    slot_offset: usize,
}

#[derive(Debug, Clone)]
struct GlobalSlot {
    offset: usize,
    ty: String,
}

#[derive(Debug, Clone, Default)]
struct GlobalLayout {
    slots: HashMap<String, GlobalSlot>,
    byte_len: usize,
}

#[derive(Debug, Clone)]
struct ByteAllocation {
    offset: usize,
    size: usize,
}

#[derive(Debug, Clone)]
struct DynamicByteAllocation {
    size_offset: usize,
}

#[derive(Debug, Clone)]
struct DataImage {
    offset: usize,
    bytes: Vec<u8>,
}

/// Zero-filled byte buffers appended after module globals.
///
/// A byte-buffer value is an `i64`/`u64` pair whose low word is its address and
/// whose high word is its allocation length. Keeping the length in the normal
/// wide-value representation lets checked buffers cross ordinary moves, calls,
/// returns, and global storage without a second ABI.
#[derive(Debug, Clone, Default)]
struct ByteAllocationLayout {
    slots: HashMap<(String, String), ByteAllocation>,
    dynamic_slots: HashMap<(String, String), DynamicByteAllocation>,
    images: HashMap<String, DataImage>,
    heap_cursor_offset: Option<usize>,
    byte_len: usize,
}

impl ByteAllocationLayout {
    fn collect(
        functions: &[&ModuleFunction<'_>],
        initial_offset: usize,
    ) -> Result<Self, BackendError> {
        let mut layout = Self {
            slots: HashMap::new(),
            dynamic_slots: HashMap::new(),
            images: HashMap::new(),
            heap_cursor_offset: None,
            byte_len: initial_offset,
        };
        for function in functions {
            let mut constants = HashMap::new();
            for instr in function.cir {
                if let (Some(destination), Some(CIROperand::Int(value))) =
                    (instr.dest.as_ref(), instr.srcs.first())
                {
                    if instr.op.starts_with("const_") {
                        constants.insert(destination.clone(), *value);
                    }
                }
                if instr.op == "str_const" {
                    let Some(CIROperand::Var(literal)) = instr.srcs.first() else {
                        return Err(BackendError::InFunction {
                            function: function.context.name.to_owned(),
                            error: Box::new(BackendError::InvalidOperand(
                                "str_const srcs[0] must be Var(literal)".to_owned(),
                            )),
                        });
                    };
                    if instr.dest.is_none() || instr.ty != "str" {
                        return Err(BackendError::InFunction {
                            function: function.context.name.to_owned(),
                            error: Box::new(BackendError::InvalidOperand(
                                "str_const requires a str destination".to_owned(),
                            )),
                        });
                    }
                    if !layout.images.contains_key(literal) {
                        let bytes = literal.as_bytes().to_vec();
                        let offset = layout.byte_len;
                        layout.byte_len = layout
                            .byte_len
                            .checked_add(bytes.len())
                            .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))?;
                        layout
                            .images
                            .insert(literal.clone(), DataImage { offset, bytes });
                    }
                }
                if instr.op != "alloc_bytes" {
                    continue;
                }
                let destination = instr.dest.as_ref().ok_or_else(|| BackendError::InFunction {
                    function: function.context.name.to_owned(),
                    error: Box::new(BackendError::InvalidOperand(
                        "alloc_bytes requires a dest".to_owned(),
                    )),
                })?;
                if !matches!(instr.ty.as_str(), "i64" | "u64") {
                    return Err(BackendError::InFunction {
                        function: function.context.name.to_owned(),
                        error: Box::new(BackendError::UnsupportedType(instr.ty.clone())),
                    });
                }
                let Some(CIROperand::Var(size_name)) = instr.srcs.first() else {
                    return Err(BackendError::InFunction {
                        function: function.context.name.to_owned(),
                        error: Box::new(BackendError::InvalidOperand(
                            "alloc_bytes srcs[0] must be a prior integer const Var".to_owned(),
                        )),
                    });
                };
                let key = (function.context.name.to_owned(), destination.clone());
                if layout.slots.contains_key(&key) || layout.dynamic_slots.contains_key(&key) {
                    return Err(BackendError::InFunction {
                        function: function.context.name.to_owned(),
                        error: Box::new(BackendError::InvalidOperand(format!(
                            "alloc_bytes destination {destination:?} is declared more than once"
                        ))),
                    });
                }
                if let Some(size) = constants.get(size_name).copied() {
                    let size = usize::try_from(size).map_err(|_| BackendError::InFunction {
                        function: function.context.name.to_owned(),
                        error: Box::new(BackendError::InvalidOperand(
                            "alloc_bytes size must be non-negative".to_owned(),
                        )),
                    })?;
                    let offset = layout.byte_len;
                    layout.byte_len = layout
                        .byte_len
                        .checked_add(size)
                        .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))?;
                    layout.slots.insert(key, ByteAllocation { offset, size });
                } else {
                    layout.byte_len = align_data_word(layout.byte_len)?;
                    let size_offset = layout.byte_len;
                    layout.byte_len = layout
                        .byte_len
                        .checked_add(4)
                        .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))?;
                    layout
                        .dynamic_slots
                        .insert(key, DynamicByteAllocation { size_offset });
                }
            }
        }
        if !layout.dynamic_slots.is_empty() {
            layout.byte_len = align_data_word(layout.byte_len)?;
            layout.heap_cursor_offset = Some(layout.byte_len);
            layout.byte_len = layout
                .byte_len
                .checked_add(4)
                .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))?;
        }
        Ok(layout)
    }
}

impl GlobalLayout {
    fn collect(functions: &[&ModuleFunction<'_>]) -> Result<Self, BackendError> {
        let mut layout = Self::default();
        for function in functions {
            for instr in function.cir {
                if !matches!(instr.op.as_str(), "global_load" | "global_store") {
                    continue;
                }
                let Some(CIROperand::Var(name)) = instr.srcs.first() else {
                    return Err(BackendError::InFunction {
                        function: function.context.name.to_owned(),
                        error: Box::new(BackendError::InvalidOperand(format!(
                            "{} srcs[0] must be Var(global_name)",
                            instr.op
                        ))),
                    });
                };
                if !is_rv32_value_type(&instr.ty) {
                    return Err(BackendError::InFunction {
                        function: function.context.name.to_owned(),
                        error: Box::new(unsupported_type_error(
                            &instr.ty,
                            &format!("{} global {name:?}", instr.op),
                        )),
                    });
                }
                if let Some(slot) = layout.slots.get(name) {
                    if slot.ty != instr.ty {
                        return Err(BackendError::InFunction {
                            function: function.context.name.to_owned(),
                            error: Box::new(BackendError::InvalidOperand(format!(
                                "global {name:?} has incompatible storage types {:?} and {:?}",
                                slot.ty, instr.ty
                            ))),
                        });
                    }
                    continue;
                }
                let offset = layout.byte_len;
                layout.byte_len = layout
                    .byte_len
                    .checked_add(8)
                    .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))?;
                layout.slots.insert(
                    name.clone(),
                    GlobalSlot {
                        offset,
                        ty: instr.ty.clone(),
                    },
                );
            }
        }
        Ok(layout)
    }
}

impl Lowerer {
    fn new(
        ctx: &FunctionContext<'_>,
        cir: &[CIRInstr],
        allow_direct_calls: bool,
        call_signatures: Option<&HashMap<String, FunctionSignature>>,
        global_layout: Option<&GlobalLayout>,
        allocation_layout: Option<&ByteAllocationLayout>,
        canonicalize_wide_return: bool,
    ) -> Result<Self, BackendError> {
        let mut env = Vec::with_capacity(ctx.params.len());
        let mut next_argument = 0;
        for (name, ty) in ctx.params {
            if !is_rv32_value_type(ty) {
                return Err(unsupported_type_error(ty, &format!("parameter {name:?}")));
            }
            let location = if is_pair_type(ty) {
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
                if is_pair_type(&instr.ty) {
                    2
                } else {
                    1
                }
            })
            .sum();
        let direct_call_argument_words = match (allow_direct_calls, call_signatures) {
            (true, Some(signatures)) => max_call_argument_words(cir, signatures)?,
            _ => 0,
        };
        let call_argument_words = direct_call_argument_words.max(max_host_argument_words(cir));
        let call_save_words = max_call_save_words(ctx, cir);
        let f64_home_names = f64_home_names(ctx, cir);
        let f64_home_words = f64_home_names.len() * 2;
        let needs_return_address_slot = allow_direct_calls && cir.iter().any(|instr| instr.op == "call");
        let frame_words = value_word_count
            + call_argument_words
            + call_save_words
            + f64_home_words
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
        let mut f64_homes = HashMap::with_capacity(f64_home_names.len());
        let mut home_slot = call_argument_words + call_save_words;
        for name in f64_home_names {
            let lo_offset = (home_slot * 4) as i32;
            f64_homes.insert(name, ValueLocation::PairSpill { lo_offset, hi_offset: lo_offset + 4 });
            home_slot += 2;
        }
        for (name, location) in &env {
            let Some(ValueLocation::PairSpill { lo_offset, hi_offset }) = f64_homes.get(name) else {
                continue;
            };
            let ValueLocation::Pair { lo, hi } = location else {
                unreachable!("f64 parameters always arrive in a register pair")
            };
            words.push(encode_sw(*lo, STACK_POINTER, *lo_offset));
            words.push(encode_sw(*hi, STACK_POINTER, *hi_offset));
        }
        Ok(Self {
            words,
            env,
            word_sized_values: HashSet::new(),
            labels: HashMap::new(),
            branches: Vec::new(),
            calls: Vec::new(),
            global_layout: global_layout.cloned(),
            allocation_layout: allocation_layout.cloned(),
            function_name: ctx.name.to_owned(),
            pending_globals: Vec::new(),
            pending_reassignment: None,
            remaining_uses: count_value_uses(cir),
            loop_carried_values: loop_carried_values(cir),
            allow_direct_calls,
            call_signatures: call_signatures.cloned().unwrap_or_default(),
            canonicalize_wide_return,
            next_internal_label: 0,
            frame_size,
            return_address_offset,
            call_argument_words,
            call_save_words,
            f64_homes,
            next_spill_slot: home_slot,
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
            if ty == "f64" {
                let Some(CIROperand::Float(value)) = instr.srcs.first() else {
                    return Err(BackendError::InvalidOperand(
                        "const_f64 srcs[0] must be Float".to_owned(),
                    ));
                };
                let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
                    unreachable!("dest_pair always returns a pair")
                };
                let bits = value.to_bits();
                self.load_constant(lo, bits as u32);
                self.load_constant(hi, (bits >> 32) as u32);
            } else if matches!(ty, "i64" | "u64")
                && !wide_literal_fits_word(instr.srcs.first(), ty)?
            {
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

        if op == "str_const" {
            return self.lower_str_const(instr);
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

        if op == "call_builtin" {
            return self.lower_host_builtin(instr);
        }

        if op == "global_load" {
            return self.lower_global_load(instr);
        }

        if op == "global_store" {
            return self.lower_global_store(instr);
        }

        if op == "alloc_bytes" {
            return self.lower_alloc_bytes(instr);
        }

        if op == "load_byte" {
            return self.lower_load_byte(instr);
        }

        if op == "store_byte" {
            return self.lower_store_byte(instr);
        }

        if let Some(ty) = op.strip_prefix("mov_") {
            self.require_scalar_type(ty, op)?;
            return self.lower_move(instr, ty, op);
        }

        for family in ["add", "sub", "mul", "div", "mod", "and", "or", "xor", "shl", "shr"] {
            if let Some(ty) = op.strip_prefix(&format!("{family}_")) {
                if ty == "f64" {
                    let service = match family {
                        "add" => HOST_ECALL_F64_ADD,
                        "sub" => HOST_ECALL_F64_SUB,
                        "mul" => HOST_ECALL_F64_MUL,
                        "div" => HOST_ECALL_F64_DIV,
                        _ => return Err(BackendError::UnsupportedOp(op.to_owned())),
                    };
                    return self.lower_f64_binary(instr, op, service);
                }
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
            if ty == "f64" {
                return self.lower_f64_comparison(instr, op, relation);
            }
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

        if op == "int_to_real" {
            return self.lower_i64_to_f64(instr);
        }

        if op == "real_to_int_trunc" {
            return self.lower_f64_to_i64_trunc(instr);
        }

        if op == "real_to_int_floor" {
            return self.lower_f64_to_i64_floor(instr);
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

    fn resolve_data_addresses(&mut self, data_offset: usize) -> Result<(), BackendError> {
        for global in self.pending_globals.clone() {
            let address = data_offset
                .checked_add(global.slot_offset)
                .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))?;
            let address = i32::try_from(address)
                .map_err(|_| BackendError::ImmediateOutOfRange(address as i64))?;
            self.load_constant_fixed_at(global.word_index, SCRATCH_REGISTER, address);
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
        // Save live caller values before marshalling outgoing arguments. An
        // incoming parameter may already occupy a0/a1, which argument loading
        // is about to overwrite.
        let saved_values = self.save_live_values_across_call(instr);

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
        self.calls.push(PendingCall {
            word_index: self.words.len(),
            function: function.clone(),
        });
        self.words.push(0);
        let return_type = if instr.ty == "any" {
            signature.return_type
        } else {
            instr.ty.clone()
        };
        let result = match (instr.dest.as_deref(), return_type.as_str()) {
            (None, "void") => Ok(()),
            (Some(_), "void") => Err(BackendError::InvalidOperand(
                "void call must not have a destination".to_owned(),
            )),
            (Some(_), ty) if is_pair_type(ty) => {
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
        };
        // Capture a callee result from a0/a1 before restoring any live caller
        // parameter that originally occupied those ABI registers.
        self.restore_live_values_after_call(&saved_values);
        result
    }

    fn lower_host_builtin(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        let Some(CIROperand::Var(name)) = instr.srcs.first() else {
            return Err(BackendError::InvalidOperand(
                "call_builtin srcs[0] must be Var(builtin_name)".to_owned(),
            ));
        };
        let saved_values = self.save_live_values_across_call(instr);
        match name.as_str() {
            "print_i64" => {
                if instr.dest.is_some() || instr.ty != "void" || instr.srcs.len() != 2 {
                    return Err(BackendError::InvalidOperand(
                        "call_builtin print_i64 requires one argument and returns void".to_owned(),
                    ));
                }
                let value = self.wide_var_location(instr, 1, "call_builtin print_i64")?;
                self.words.push(encode_addi(A0, value.low(), 0));
                self.copy_or_extend_high(A1, value, true);
                self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_WRITE_I64);
            }
            "putchar" => {
                if instr.dest.is_some() || instr.ty != "void" || instr.srcs.len() != 2 {
                    return Err(BackendError::InvalidOperand(
                        "call_builtin putchar requires one argument and returns void".to_owned(),
                    ));
                }
                let value = self.var_src(instr, 1, "call_builtin putchar")?;
                self.words.push(encode_addi(A0, value, 0));
                self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_WRITE_BYTE);
            }
            "print_f64" => {
                if instr.dest.is_some() || instr.ty != "void" || instr.srcs.len() != 2 {
                    return Err(BackendError::InvalidOperand(
                        "call_builtin print_f64 requires one f64 argument and returns void"
                            .to_owned(),
                    ));
                }
                self.stage_f64_argument(instr, 1, "call_builtin print_f64", 0)?;
                self.load_host_argument_pair(A0, A1, 0);
                self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_WRITE_F64);
            }
            "getchar" => {
                if instr.dest.is_none() || instr.ty != "i64" || instr.srcs.len() != 1 {
                    return Err(BackendError::InvalidOperand(
                        "call_builtin getchar takes no arguments and returns i64".to_owned(),
                    ));
                }
                self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_READ_BYTE);
                self.words.push(encode_ecall());
                let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "call_builtin getchar")? else {
                    unreachable!("getchar returns an i64 pair")
                };
                self.words.push(encode_addi(lo, A0, 0));
                self.words.push(encode_addi(hi, A1, 0));
                self.restore_live_values_after_call(&saved_values);
                return Ok(());
            }
            _ => return Err(BackendError::UnsupportedOp(format!("call_builtin {name}"))),
        }
        self.words.push(encode_ecall());
        self.restore_live_values_after_call(&saved_values);
        Ok(())
    }

    fn lower_f64_binary(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        service: u32,
    ) -> Result<(), BackendError> {
        let saved_values = self.save_live_values_across_call(instr);
        self.stage_f64_argument(instr, 0, op, 0)?;
        self.stage_f64_argument(instr, 1, op, 2)?;
        self.load_host_argument_pair(A0, A1, 0);
        self.load_host_argument_pair(A2, A3, 2);
        self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, service);
        self.words.push(encode_ecall());
        self.words.push(encode_sw(A0, STACK_POINTER, 0));
        self.words.push(encode_sw(A1, STACK_POINTER, 4));
        self.restore_live_values_after_call(&saved_values);
        (|| {
            let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, op)? else {
                unreachable!("f64 results always use a pair")
            };
            self.words.push(encode_lw(lo, STACK_POINTER, 0));
            self.words.push(encode_lw(hi, STACK_POINTER, 4));
            Ok(())
        })()
    }

    fn lower_f64_comparison(
        &mut self,
        instr: &CIRInstr,
        op: &str,
        relation: &str,
    ) -> Result<(), BackendError> {
        let saved_values = self.save_live_values_across_call(instr);
        self.stage_f64_argument(instr, 0, op, 0)?;
        self.stage_f64_argument(instr, 1, op, 2)?;
        self.load_host_argument_pair(A0, A1, 0);
        self.load_host_argument_pair(A2, A3, 2);
        self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_F64_CMP);
        self.words.push(encode_ecall());
        self.words.push(encode_sw(A0, STACK_POINTER, 0));
        self.restore_live_values_after_call(&saved_values);
        (|| {
            let destination = self.dest(instr, op)?;
            self.words.push(encode_lw(destination, STACK_POINTER, 0));
            match relation {
                "eq" => self.words.push(riscv_encoder::encode_sltiu(destination, destination, 1)),
                "ne" => self.words.push(encode_sltu(destination, X0_ZERO, destination)),
                "lt" => self.words.push(encode_slt(destination, destination, X0_ZERO)),
                "gt" => self.words.push(encode_slt(destination, X0_ZERO, destination)),
                "le" => {
                    self.words.push(encode_slt(destination, X0_ZERO, destination));
                    self.words.push(encode_xori(destination, destination, 1));
                }
                "ge" => {
                    self.words.push(encode_slt(destination, destination, X0_ZERO));
                    self.words.push(encode_xori(destination, destination, 1));
                }
                _ => return Err(BackendError::UnsupportedOp(op.to_owned())),
            }
            Ok(())
        })()
    }

    fn lower_i64_to_f64(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.ty != "f64" || instr.srcs.len() != 1 {
            return Err(BackendError::InvalidOperand(
                "int_to_real requires one i64 source and an f64 destination".to_owned(),
            ));
        }
        let saved_values = self.save_live_values_across_call(instr);
        let value = self.wide_var_location(instr, 0, "int_to_real")?;
        self.words.push(encode_sw(value.low(), STACK_POINTER, 0));
        self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, value, true);
        self.words
            .push(encode_sw(SECOND_SCRATCH_REGISTER, STACK_POINTER, 4));
        self.load_host_argument_pair(A0, A1, 0);
        self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_I64_TO_F64);
        self.words.push(encode_ecall());
        self.words.push(encode_sw(A0, STACK_POINTER, 0));
        self.words.push(encode_sw(A1, STACK_POINTER, 4));
        self.restore_live_values_after_call(&saved_values);
        (|| {
            let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "int_to_real")? else {
                unreachable!("f64 results always use a pair")
            };
            self.words.push(encode_lw(lo, STACK_POINTER, 0));
            self.words.push(encode_lw(hi, STACK_POINTER, 4));
            Ok(())
        })()
    }

    fn lower_f64_to_i64_trunc(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.ty != "i64" || instr.srcs.len() != 1 {
            return Err(BackendError::InvalidOperand(
                "real_to_int_trunc requires one f64 source and an i64 destination".to_owned(),
            ));
        }
        let saved_values = self.save_live_values_across_call(instr);
        self.stage_f64_argument(instr, 0, "real_to_int_trunc", 0)?;
        self.load_host_argument_pair(A0, A1, 0);
        self.load_constant(
            HOST_ECALL_SERVICE_REGISTER as u32,
            HOST_ECALL_F64_TO_I64_TRUNC,
        );
        self.words.push(encode_ecall());
        self.words.push(encode_sw(A0, STACK_POINTER, 0));
        self.words.push(encode_sw(A1, STACK_POINTER, 4));
        self.restore_live_values_after_call(&saved_values);
        (|| {
            let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "real_to_int_trunc")?
            else {
                unreachable!("i64 results always use a pair")
            };
            self.words.push(encode_lw(lo, STACK_POINTER, 0));
            self.words.push(encode_lw(hi, STACK_POINTER, 4));
            Ok(())
        })()
    }

    fn lower_f64_to_i64_floor(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.ty != "i64" || instr.srcs.len() != 1 {
            return Err(BackendError::InvalidOperand(
                "real_to_int_floor requires one f64 source and an i64 destination".to_owned(),
            ));
        }

        let floor_saved_values = self.save_live_values_across_call(instr);
        self.stage_f64_argument(instr, 0, "real_to_int_floor", 0)?;
        self.load_host_argument_pair(A0, A1, 0);
        self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_F64_FLOOR);
        self.words.push(encode_ecall());
        self.words.push(encode_sw(A0, STACK_POINTER, 0));
        self.words.push(encode_sw(A1, STACK_POINTER, 4));
        self.restore_live_values_after_call(&floor_saved_values);

        let conversion_saved_values = self.save_live_values_across_call(instr);
        self.load_host_argument_pair(A0, A1, 0);
        self.load_constant(
            HOST_ECALL_SERVICE_REGISTER as u32,
            HOST_ECALL_F64_TO_I64_TRUNC,
        );
        self.words.push(encode_ecall());
        self.words.push(encode_sw(A0, STACK_POINTER, 0));
        self.words.push(encode_sw(A1, STACK_POINTER, 4));
        self.restore_live_values_after_call(&conversion_saved_values);
        (|| {
            let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "real_to_int_floor")?
            else {
                unreachable!("i64 results always use a pair")
            };
            self.words.push(encode_lw(lo, STACK_POINTER, 0));
            self.words.push(encode_lw(hi, STACK_POINTER, 4));
            Ok(())
        })()
    }

    fn stage_f64_argument(
        &mut self,
        instr: &CIRInstr,
        index: usize,
        op: &str,
        word_index: usize,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { lo, hi } = self.wide_var_location(instr, index, op)? else {
            return Err(BackendError::InvalidOperand(format!(
                "{op} srcs[{index}] must be an f64 pair"
            )));
        };
        let offset = (word_index * 4) as i32;
        self.words.push(encode_sw(lo, STACK_POINTER, offset));
        self.words.push(encode_sw(hi, STACK_POINTER, offset + 4));
        Ok(())
    }

    fn load_host_argument_pair(
        &mut self,
        low_register: u32,
        high_register: u32,
        word_index: usize,
    ) {
        let offset = (word_index * 4) as i32;
        self.words.push(encode_lw(low_register, STACK_POINTER, offset));
        self.words
            .push(encode_lw(high_register, STACK_POINTER, offset + 4));
    }

    fn lower_global_load(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.srcs.len() != 1 {
            return Err(BackendError::InvalidOperand(format!(
                "global_load requires one global name, got {} operands",
                instr.srcs.len()
            )));
        }
        let slot = self.global_slot(instr, "global_load")?;
        if matches!(slot.ty.as_str(), "i64" | "u64" | "str") {
            let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "global_load")? else {
                unreachable!("dest_pair always returns a pair")
            };
            self.reserve_global_address(slot.offset);
            self.words.push(encode_lw(lo, SCRATCH_REGISTER, 0));
            self.words.push(encode_lw(hi, SCRATCH_REGISTER, 4));
        } else {
            let destination = self.dest(instr, "global_load")?;
            self.reserve_global_address(slot.offset);
            self.words.push(encode_lw(destination, SCRATCH_REGISTER, 0));
            self.mask_unsigned(destination, &slot.ty);
        }
        Ok(())
    }

    fn lower_global_store(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.dest.is_some() {
            return Err(BackendError::InvalidOperand(
                "global_store must not have a destination".to_owned(),
            ));
        }
        if instr.srcs.len() != 2 {
            return Err(BackendError::InvalidOperand(format!(
                "global_store requires a global name and one value, got {} operands",
                instr.srcs.len()
            )));
        }
        let slot = self.global_slot(instr, "global_store")?;
        if matches!(slot.ty.as_str(), "i64" | "u64" | "str") {
            let source = self.wide_var_location(instr, 1, "global_store")?;
            self.reserve_global_address(slot.offset);
            self.words.push(encode_sw(source.low(), SCRATCH_REGISTER, 0));
            self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, source, is_signed(&slot.ty));
            self.words
                .push(encode_sw(SECOND_SCRATCH_REGISTER, SCRATCH_REGISTER, 4));
        } else {
            let source = self.var_src(instr, 1, "global_store")?;
            self.reserve_global_address(slot.offset);
            self.words.push(encode_sw(source, SCRATCH_REGISTER, 0));
        }
        Ok(())
    }

    fn lower_alloc_bytes(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.srcs.len() != 1 {
            return Err(BackendError::InvalidOperand(format!(
                "alloc_bytes requires one size operand, got {}",
                instr.srcs.len()
            )));
        }
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "alloc_bytes")? else {
            unreachable!("alloc_bytes requires an i64/u64 destination")
        };
        if let Some(slot) = self.byte_allocation(instr)? {
            self.reserve_data_address(slot.offset);
            self.words.push(encode_addi(lo, SCRATCH_REGISTER, 0));
            self.load_constant(hi, slot.size as u32);
            return Ok(());
        }
        let dynamic = self.dynamic_byte_allocation(instr, "alloc_bytes")?;
        let size = self.wide_var_location(instr, 0, "alloc_bytes")?;
        self.guard_high_word_is_zero(size);
        let cursor_offset = self.heap_cursor_offset()?;
        self.reserve_data_address(cursor_offset);
        self.words
            .push(encode_lw(SECOND_SCRATCH_REGISTER, SCRATCH_REGISTER, 0));
        self.words
            .push(encode_add(DIVISION_TEMP_REGISTER, SECOND_SCRATCH_REGISTER, size.low()));
        self.words.push(encode_sltu(
            DIVISION_BORROW_REGISTER,
            DIVISION_TEMP_REGISTER,
            SECOND_SCRATCH_REGISTER,
        ));
        let no_overflow = self.internal_label("alloc_no_overflow");
        self.record_named_branch(
            no_overflow.clone(),
            BranchKind::EqZero {
                rs1: DIVISION_BORROW_REGISTER,
            },
        );
        self.emit_memory_exit();
        self.mark_label(no_overflow);
        self.load_constant(DIVISION_BORROW_REGISTER, (DEFAULT_MEMORY_SIZE - 15) as u32);
        self.words.push(encode_sltu(
            SCRATCH_REGISTER,
            DIVISION_TEMP_REGISTER,
            DIVISION_BORROW_REGISTER,
        ));
        let fits_memory = self.internal_label("alloc_fits_memory");
        self.record_named_branch(
            fits_memory.clone(),
            BranchKind::NeZero {
                rs1: SCRATCH_REGISTER,
            },
        );
        self.emit_memory_exit();
        self.mark_label(fits_memory);
        self.reserve_data_address(cursor_offset);
        self.words
            .push(encode_sw(DIVISION_TEMP_REGISTER, SCRATCH_REGISTER, 0));
        self.reserve_data_address(dynamic.size_offset);
        self.words.push(encode_sw(size.low(), SCRATCH_REGISTER, 0));
        self.words
            .push(encode_addi(lo, SECOND_SCRATCH_REGISTER, 0));
        self.words.push(encode_addi(hi, size.low(), 0));
        Ok(())
    }

    fn lower_str_const(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.ty != "str" || instr.srcs.len() != 1 {
            return Err(BackendError::InvalidOperand(
                "str_const requires one literal operand and a str destination".to_owned(),
            ));
        }
        let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "str_const")? else {
            unreachable!("str_const uses an address/length pair")
        };
        let image = self.data_image(instr)?;
        self.reserve_data_address(image.offset);
        self.words.push(encode_addi(lo, SCRATCH_REGISTER, 0));
        self.load_constant(hi, image.bytes.len() as u32);
        Ok(())
    }

    fn lower_load_byte(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.srcs.len() != 2 {
            return Err(BackendError::InvalidOperand(format!(
                "load_byte requires base and offset operands, got {}",
                instr.srcs.len()
            )));
        }
        if instr.ty == "str" {
            return Err(BackendError::UnsupportedType("str".to_owned()));
        }
        self.require_scalar_type(&instr.ty, "load_byte")?;
        let base = self.wide_var_location(instr, 0, "load_byte")?;
        let offset = self.wide_var_location(instr, 1, "load_byte")?;
        self.guard_byte_access(base, offset, "load_byte")?;
        self.words
            .push(encode_add(SCRATCH_REGISTER, base.low(), offset.low()));
        if matches!(instr.ty.as_str(), "i64" | "u64") {
            let ValueLocation::Pair { lo, hi } = self.dest_pair(instr, "load_byte")? else {
                unreachable!("load_byte wide destination uses a register pair")
            };
            self.words.push(encode_lbu(lo, SCRATCH_REGISTER, 0));
            self.words.push(encode_addi(hi, X0_ZERO, 0));
        } else {
            let destination = self.dest(instr, "load_byte")?;
            self.words.push(encode_lbu(destination, SCRATCH_REGISTER, 0));
            self.mask_unsigned(destination, &instr.ty);
        }
        Ok(())
    }

    fn lower_store_byte(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        if instr.dest.is_some() {
            return Err(BackendError::InvalidOperand(
                "store_byte must not have a destination".to_owned(),
            ));
        }
        if instr.srcs.len() != 3 {
            return Err(BackendError::InvalidOperand(format!(
                "store_byte requires base, offset, and value operands, got {}",
                instr.srcs.len()
            )));
        }
        let base = self.wide_var_location(instr, 0, "store_byte")?;
        let offset = self.wide_var_location(instr, 1, "store_byte")?;
        let value = self.var_src(instr, 2, "store_byte")?;
        self.guard_byte_access(base, offset, "store_byte")?;
        self.words
            .push(encode_add(SCRATCH_REGISTER, base.low(), offset.low()));
        self.words.push(encode_sb(value, SCRATCH_REGISTER, 0));
        Ok(())
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
        if !is_pair_type(ty) {
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
        match self.source_location(name)? {
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
        self.source_location(name)
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
        if let Some(location) = self
            .env
            .iter()
            .find_map(|(existing, location)| (existing == name).then_some(*location))
        {
            return match location {
                ValueLocation::Word(reg) => Ok(reg),
                ValueLocation::Pair { .. } => Err(BackendError::InvalidOperand(format!(
                    "{name:?} is already bound as a 64-bit value"
                ))),
                ValueLocation::Spill { .. } => {
                    let register = self.allocate_value_register()?;
                    self.reassign(name, location, ValueLocation::Word(register));
                    Ok(register)
                }
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
        if let Some(location) = self
            .env
            .iter()
            .find_map(|(existing, location)| (existing == name).then_some(*location))
        {
            return match location {
                ValueLocation::Pair { lo, hi } => Ok(ValueLocation::Pair { lo, hi }),
                ValueLocation::Word(_) | ValueLocation::Spill { .. } | ValueLocation::PairSpill { .. } => {
                    let destination = self.allocate_pair_registers()?;
                    self.reassign(name, location, destination);
                    Ok(destination)
                }
            };
        }
        let location = self.allocate_pair_registers()?;
        self.env.push((name.to_owned(), location));
        Ok(location)
    }

    fn reassign(&mut self, name: &str, old_location: ValueLocation, new_location: ValueLocation) {
        debug_assert!(self.pending_reassignment.is_none());
        for (existing, location) in &mut self.env {
            if existing == name {
                *location = new_location;
            }
        }
        self.pending_reassignment = Some(PendingReassignment {
            name: name.to_owned(),
            old_location,
        });
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
                    && self.remaining_uses.get(name).copied().unwrap_or_default() == 0
                    && !self.loop_carried_values.contains(name))
        });
    }

    fn release_dead_pair_values(&mut self) {
        self.env.retain(|(name, location)| {
            !matches!(location, ValueLocation::Pair { .. }
                if self.remaining_uses.get(name).copied().unwrap_or_default() == 0
                    && !self.loop_carried_values.contains(name))
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
        let needs_fallback_slot = self.env.iter().any(|(name, location)| {
            matches!(location, ValueLocation::Pair { lo: pair_lo, hi: pair_hi }
                if *pair_lo == lo && *pair_hi == hi)
                && !self.f64_homes.contains_key(name)
        });
        let fallback_slot = needs_fallback_slot.then(|| {
            let lo_offset = (self.next_spill_slot * 4) as i32;
            self.next_spill_slot += 2;
            self.words.push(encode_sw(lo, STACK_POINTER, lo_offset));
            self.words.push(encode_sw(hi, STACK_POINTER, lo_offset + 4));
            ValueLocation::PairSpill { lo_offset, hi_offset: lo_offset + 4 }
        });
        for (name, location) in &mut self.env {
            if matches!(location, ValueLocation::Pair { lo: pair_lo, hi: pair_hi }
                if *pair_lo == lo && *pair_hi == hi)
            {
                *location = self.f64_homes.get(name).copied().unwrap_or_else(|| {
                    fallback_slot.expect("pair values need a fallback slot")
                });
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
            ValueLocation::Spill { offset } => {
                let register = if index == 0 {
                    SPILLED_LHS_REGISTER
                } else {
                    DIVISION_TEMP_REGISTER
                };
                self.words.push(encode_lw(register, STACK_POINTER, offset));
                Ok(ValueLocation::Word(register))
            }
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
            ValueLocation::Spill { offset } => {
                let register = if index == 0 {
                    SPILLED_LHS_REGISTER
                } else {
                    DIVISION_DIVISOR_LOW_REGISTER
                };
                self.words.push(encode_lw(register, STACK_POINTER, offset));
                Ok(ValueLocation::Word(register))
            }
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

    fn source_location(&self, name: &str) -> Result<ValueLocation, BackendError> {
        if let Some(reassignment) = &self.pending_reassignment {
            if reassignment.name == name {
                return Ok(reassignment.old_location);
            }
        }
        self.lookup_location(name)
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
        if let Some(reassignment) = self.pending_reassignment.take() {
            if let Some(home) = self.f64_homes.get(&reassignment.name).copied() {
                for (name, location) in &mut self.env {
                    if *name == reassignment.name {
                        *location = home;
                    }
                }
            }
        }
    }

    fn store_f64_home(&mut self, instr: &CIRInstr) -> Result<(), BackendError> {
        let Some(name) = instr.dest.as_deref() else { return Ok(()); };
        let Some(ValueLocation::PairSpill { lo_offset, hi_offset }) = self.f64_homes.get(name).copied() else { return Ok(()); };
        match self.lookup_location(name)? {
            ValueLocation::Pair { lo, hi } => {
                self.words.push(encode_sw(lo, STACK_POINTER, lo_offset));
                self.words.push(encode_sw(hi, STACK_POINTER, hi_offset));
            }
            ValueLocation::Word(register) => {
                self.words.push(encode_sw(register, STACK_POINTER, lo_offset));
                self.words.push(if is_signed(&instr.ty) { encode_srai(SCRATCH_REGISTER, register, 31) } else { encode_addi(SCRATCH_REGISTER, X0_ZERO, 0) });
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, hi_offset));
            }
            ValueLocation::PairSpill { lo_offset: source_lo, hi_offset: source_hi } => {
                self.words.push(encode_lw(SCRATCH_REGISTER, STACK_POINTER, source_lo));
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, lo_offset));
                self.words.push(encode_lw(SCRATCH_REGISTER, STACK_POINTER, source_hi));
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, hi_offset));
            }
            ValueLocation::Spill { offset } => {
                self.words.push(encode_lw(SCRATCH_REGISTER, STACK_POINTER, offset));
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, lo_offset));
                self.words.push(if is_signed(&instr.ty) { encode_srai(SCRATCH_REGISTER, SCRATCH_REGISTER, 31) } else { encode_addi(SCRATCH_REGISTER, X0_ZERO, 0) });
                self.words.push(encode_sw(SCRATCH_REGISTER, STACK_POINTER, hi_offset));
            }
        }
        for (existing, location) in &mut self.env {
            if existing == name { *location = ValueLocation::PairSpill { lo_offset, hi_offset }; }
        }
        Ok(())
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
        self.words.push(encode_addi(DIVISION_COUNTER_REGISTER, rhs.low(), 0));
        let rhs_lo = DIVISION_COUNTER_REGISTER;

        // (a_hi * 2^32 + a_lo) * (b_hi * 2^32 + b_lo), modulo 2^64.
        // Only the low word of each cross product contributes to the result.
        self.copy_or_extend_high(SECOND_SCRATCH_REGISTER, lhs, signed);
        self.copy_or_extend_high(SCRATCH_REGISTER, rhs, signed);
        self.words.push(encode_mul(lo, lhs.low(), rhs_lo));
        self.words.push(encode_mulhu(hi, lhs.low(), rhs_lo));
        self.words.push(encode_mul(SCRATCH_REGISTER, lhs.low(), SCRATCH_REGISTER));
        self.words.push(encode_add(hi, hi, SCRATCH_REGISTER));
        self.words.push(encode_mul(SCRATCH_REGISTER, SECOND_SCRATCH_REGISTER, rhs_lo));
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

    fn global_slot(&self, instr: &CIRInstr, op: &str) -> Result<GlobalSlot, BackendError> {
        let Some(layout) = &self.global_layout else {
            return Err(BackendError::UnsupportedOp(format!(
                "{op} (module linking required)"
            )));
        };
        let Some(CIROperand::Var(name)) = instr.srcs.first() else {
            return Err(BackendError::InvalidOperand(format!(
                "{op} srcs[0] must be Var(global_name)"
            )));
        };
        let slot = layout
            .slots
            .get(name)
            .cloned()
            .ok_or_else(|| BackendError::InvalidOperand(format!(
                "{op}: global {name:?} has no allocated storage"
            )))?;
        if slot.ty != instr.ty {
            return Err(BackendError::InvalidOperand(format!(
                "{op}: global {name:?} has storage type {:?}, not {:?}",
                slot.ty, instr.ty
            )));
        }
        Ok(slot)
    }

    fn byte_allocation(&self, instr: &CIRInstr) -> Result<Option<ByteAllocation>, BackendError> {
        let Some(layout) = &self.allocation_layout else {
            return Err(BackendError::UnsupportedOp(
                "alloc_bytes (module linking required)".to_owned(),
            ));
        };
        let destination = instr.dest.as_ref().ok_or_else(|| {
            BackendError::InvalidOperand("alloc_bytes requires a dest".to_owned())
        })?;
        Ok(layout
            .slots
            .get(&(self.function_name.clone(), destination.clone()))
            .cloned())
    }

    fn dynamic_byte_allocation(
        &self,
        instr: &CIRInstr,
        op: &str,
    ) -> Result<DynamicByteAllocation, BackendError> {
        let Some(layout) = &self.allocation_layout else {
            return Err(BackendError::UnsupportedOp(format!(
                "{op} (module linking required)"
            )));
        };
        let destination = instr.dest.as_ref().ok_or_else(|| {
            BackendError::InvalidOperand(format!("{op} requires a dest"))
        })?;
        layout
            .dynamic_slots
            .get(&(self.function_name.clone(), destination.clone()))
            .cloned()
            .ok_or_else(|| {
                BackendError::InvalidOperand(format!(
                    "{op}: destination {destination:?} has no dynamic allocation"
                ))
            })
    }

    fn data_image(&self, instr: &CIRInstr) -> Result<DataImage, BackendError> {
        let Some(layout) = &self.allocation_layout else {
            return Err(BackendError::UnsupportedOp(
                "str_const (module linking required)".to_owned(),
            ));
        };
        let Some(CIROperand::Var(literal)) = instr.srcs.first() else {
            return Err(BackendError::InvalidOperand(
                "str_const srcs[0] must be Var(literal)".to_owned(),
            ));
        };
        layout.images.get(literal).cloned().ok_or_else(|| {
            BackendError::InvalidOperand(format!("str_const literal {literal:?} has no data image"))
        })
    }

    fn heap_cursor_offset(&self) -> Result<usize, BackendError> {
        self.allocation_layout
            .as_ref()
            .and_then(|layout| layout.heap_cursor_offset)
            .ok_or_else(|| BackendError::InvalidOperand("dynamic alloc_bytes has no heap cursor".to_owned()))
    }

    fn guard_byte_access(
        &mut self,
        base: ValueLocation,
        offset: ValueLocation,
        op: &str,
    ) -> Result<(), BackendError> {
        let ValueLocation::Pair { hi: length, .. } = base else {
            return Err(BackendError::InvalidOperand(format!(
                "{op} base must be an alloc_bytes descriptor"
            )));
        };
        self.guard_high_word_is_zero(offset);
        self.words.push(encode_sltu(
            SCRATCH_REGISTER,
            offset.low(),
            length,
        ));
        let in_bounds = self.internal_label("byte_in_bounds");
        self.record_named_branch(
            in_bounds.clone(),
            BranchKind::NeZero {
                rs1: SCRATCH_REGISTER,
            },
        );
        self.emit_memory_exit();
        self.mark_label(in_bounds);
        Ok(())
    }

    fn guard_high_word_is_zero(&mut self, value: ValueLocation) {
        let ValueLocation::Pair { hi, .. } = value else {
            return;
        };
        let low_word_only = self.internal_label("byte_offset_low_word");
        self.record_named_branch(low_word_only.clone(), BranchKind::EqZero { rs1: hi });
        self.emit_memory_exit();
        self.mark_label(low_word_only);
    }

    fn emit_memory_exit(&mut self) {
        self.load_constant(A0, MEMORY_BOUNDS_EXIT_CODE as u32);
        self.load_constant(HOST_ECALL_SERVICE_REGISTER as u32, HOST_ECALL_EXIT);
        self.words.push(encode_ecall());
    }

    fn reserve_global_address(&mut self, slot_offset: usize) {
        self.reserve_data_address(slot_offset);
    }

    fn reserve_data_address(&mut self, slot_offset: usize) {
        let word_index = self.words.len();
        // Keep this pair fixed-width so module-data placement can be resolved
        // after every function has been lowered.
        self.words.extend([0, 0]);
        self.pending_globals.push(PendingGlobal {
            word_index,
            slot_offset,
        });
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

    fn load_constant_fixed_at(&mut self, word_index: usize, rd: u32, value: i32) {
        let upper = ((value as i64 + 0x800) >> 12) as i32;
        let lower = value as i64 - ((upper as i64) << 12);
        self.words[word_index] = encode_lui(rd, upper as u32);
        self.words[word_index + 1] = encode_addi(rd, rd, lower as i32);
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

fn loop_carried_values(cir: &[CIRInstr]) -> HashSet<String> {
    let labels: HashMap<&str, usize> = cir.iter().enumerate().filter_map(|(index, instr)| {
        (instr.op == "label").then(|| instr.srcs.first().and_then(CIROperand::as_var).map(|name| (name, index))).flatten()
    }).collect();
    let mut values = HashSet::new();
    for (jump_index, instr) in cir.iter().enumerate() {
        let label_index = match instr.op.as_str() {
            "jmp" => instr.srcs.first().and_then(CIROperand::as_var).and_then(|name| labels.get(name)),
            "jmp_if_false" | "br_false_bool" | "jmp_if_true" | "br_true_bool" => instr.srcs.get(1).and_then(CIROperand::as_var).and_then(|name| labels.get(name)),
            _ => None,
        };
        let Some(&label_index) = label_index else { continue; };
        if label_index >= jump_index { continue; }
        for body_instr in &cir[label_index..=jump_index] {
            for (index, operand) in body_instr.srcs.iter().enumerate() {
                if is_value_source(body_instr, index) {
                    if let CIROperand::Var(name) = operand { values.insert(name.clone()); }
                }
            }
        }
    }
    values
}

fn defines_f64_pair(instr: &CIRInstr) -> bool {
    instr.dest.is_some() && instr.ty == "f64" && comparison_parts(&instr.op).is_none()
}

fn f64_home_names(ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Vec<String> {
    let loop_homes = has_backward_branch(cir);
    let mut names = Vec::new();
    for (name, ty) in ctx.params {
        if (ty == "f64" || (loop_homes && is_pair_type(ty))) && !names.contains(name) { names.push(name.clone()); }
    }
    for instr in cir {
        if let Some(name) = &instr.dest {
            if (defines_f64_pair(instr) || (loop_homes && is_pair_type(&instr.ty))) && !names.contains(name) { names.push(name.clone()); }
        }
    }
    names
}

fn has_backward_branch(cir: &[CIRInstr]) -> bool {
    let labels: HashMap<&str, usize> = cir.iter().enumerate().filter_map(|(index, instr)| {
        (instr.op == "label").then(|| instr.srcs.first().and_then(CIROperand::as_var).map(|name| (name, index))).flatten()
    }).collect();
    cir.iter().enumerate().any(|(index, instr)| {
        let label = match instr.op.as_str() {
            "jmp" => instr.srcs.first().and_then(CIROperand::as_var),
            "jmp_if_false" | "br_false_bool" | "jmp_if_true" | "br_true_bool" => instr.srcs.get(1).and_then(CIROperand::as_var),
            _ => None,
        };
        label.and_then(|name| labels.get(name)).is_some_and(|target| *target < index)
    })
}

fn abi_word_count(ty: &str) -> usize {
    usize::from(is_pair_type(ty)) + 1
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

fn max_host_argument_words(cir: &[CIRInstr]) -> usize {
    cir.iter()
        .filter_map(|instr| match instr.op.as_str() {
            "int_to_real" | "real_to_int_trunc" | "real_to_int_floor" => Some(2),
            "call_builtin"
                if instr.srcs.first().and_then(CIROperand::as_var) == Some("print_f64") =>
            {
                Some(2)
            }
            op if op.ends_with("_f64")
                && (op.starts_with("add_")
                    || op.starts_with("sub_")
                    || op.starts_with("mul_")
                    || op.starts_with("div_")
                    || op.starts_with("cmp_")) => Some(4),
            _ => None,
        })
        .max()
        .unwrap_or_default()
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
        if call.op != "call" && call.op != "call_builtin" && max_host_argument_words(std::slice::from_ref(call)) == 0 {
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
    if is_pair_type(ty) || ty == "any" {
        2
    } else {
        1
    }
}

fn align_data_word(offset: usize) -> Result<usize, BackendError> {
    offset
        .checked_add(3)
        .map(|aligned| aligned & !3)
        .ok_or(BackendError::ImmediateOutOfRange(i64::MAX))
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
        "label" | "jmp" | "str_const" => false,
        "call" | "call_builtin" => index != 0,
        "global_load" => false,
        "global_store" => index == 1,
        "alloc_bytes" => index == 0,
        "load_byte" => index < 2,
        "store_byte" => index < 3,
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
/// (double precision, adds RV32D). `f64` is therefore represented as an
/// integer pair and only the host-backed operations explicitly lowered here
/// can execute on RV32I; other float forms still need more soft-float work or
/// a different target. Saying that plainly beats a generic "unsupported type".
fn is_floating_point_type(ty: &str) -> bool {
    matches!(ty, "f16" | "f32" | "f64" | "f128")
}

/// Build the right "this type does not fit RV32I" error for `ty`.
///
/// Floats get the specific [`BackendError::UnsupportedFloat`] refusal (the
/// fix is retarget-or-add-soft-float support); everything else stays the generic
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
        "u4" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "bool" | "str" | "f64"
    )
}

/// Values represented in RV32I integer registers as low/high 32-bit pairs.
///
/// `f64` joins the existing wide integer and string ABI here. Its raw bits stay
/// opaque except at simulator host soft-float service boundaries.
fn is_pair_type(ty: &str) -> bool {
    matches!(ty, "i64" | "u64" | "str" | "f64")
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
