//! Lower `IrProgram` into `CILProgramArtifact` (CIL method bytecode).
//!
//! # Pipeline
//!
//! ```text
//! IrProgram
//!   ↓ validate_for_clr()         — pre-flight validation
//!   ↓ analyse_program()           — discover callable regions + layout
//!   ↓ lower_region() × N          — emit CIL bytecode per callable region
//!   → CILProgramArtifact          — structured multi-method artifact
//!       ↓ (future) CLR packager   — .method header + wrapping PE format
//! ```
//!
//! # Calling convention
//!
//! The CLR is a stack machine with two register files:
//!
//! - **x registers** (argument / scratch) — alias our virtual registers v0..vN
//! - **y registers** (stack-local, callee-saves) — reserved for future use
//!
//! This v1 lowering uses **only x registers** (implemented as CIL locals)
//! and emits no `allocate`/`deallocate` framing.  All IR virtual registers
//! become local variables at the same index.
//!
//! # SYSCALL numbers
//!
//! | Number | Meaning |
//! |--------|---------|
//! | 1      | write   |
//! | 2      | read    |
//! | 10     | exit    |

use std::collections::{HashMap, HashSet};

use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

use crate::builder::{CILBranchKind, CILBytecodeBuilder, encode_ldloc, encode_ldc_i4, encode_stloc};

// ===========================================================================
// Public error type
// ===========================================================================

/// Error raised when IR → CIL lowering fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CILBackendError(pub String);

impl std::fmt::Display for CILBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CILBackendError: {}", self.0)
    }
}

// ===========================================================================
// CILHelper — runtime helpers injected by the lowering pipeline
// ===========================================================================

/// Runtime helper functions required by lowered CIL code.
///
/// The CLR has no built-in memory-access or syscall instructions — those
/// operations must be implemented as static methods in a helper class that
/// the lowered code calls via `call` or `callvirt`.
///
/// This enum identifies the five helpers the lowering pipeline injects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CILHelper {
    /// `static int32 MemLoadByte(int32 addr)` — load one byte, zero-extend.
    MemLoadByte,
    /// `static void MemStoreByte(int32 addr, int32 val)` — store low 8 bits.
    MemStoreByte,
    /// `static int32 LoadWord(int32 addr)` — load one 32-bit word.
    LoadWord,
    /// `static void StoreWord(int32 addr, int32 val)` — store a 32-bit word.
    StoreWord,
    /// `static int32 Syscall(int32 num, int32 arg)` — invoke OS syscall.
    Syscall,
}

/// A descriptor for a runtime helper: name, parameter types, return type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CILHelperSpec {
    /// The method name used in the generated IL assembly.
    pub name: &'static str,
    /// Parameter type strings, in order.
    pub param_types: &'static [&'static str],
    /// Return type string (`"int32"` or `"void"`).
    pub return_type: &'static str,
}

/// Canonical helper specs, indexed by `CILHelper`.
static HELPER_SPECS: &[(CILHelper, CILHelperSpec)] = &[
    (CILHelper::MemLoadByte, CILHelperSpec {
        name: "MemLoadByte",
        param_types: &["int32"],
        return_type: "int32",
    }),
    (CILHelper::MemStoreByte, CILHelperSpec {
        name: "MemStoreByte",
        param_types: &["int32", "int32"],
        return_type: "void",
    }),
    (CILHelper::LoadWord, CILHelperSpec {
        name: "LoadWord",
        param_types: &["int32"],
        return_type: "int32",
    }),
    (CILHelper::StoreWord, CILHelperSpec {
        name: "StoreWord",
        param_types: &["int32", "int32"],
        return_type: "void",
    }),
    (CILHelper::Syscall, CILHelperSpec {
        name: "Syscall",
        param_types: &["int32", "int32"],
        return_type: "int32",
    }),
];

/// Returns the helper spec for a given `CILHelper`.
///
/// Uses an exhaustive `match` so the compiler will catch any new `CILHelper`
/// variant that hasn't been registered in `HELPER_SPECS`.
pub fn helper_spec(h: CILHelper) -> &'static CILHelperSpec {
    match h {
        CILHelper::MemLoadByte  => &HELPER_SPECS[0].1,
        CILHelper::MemStoreByte => &HELPER_SPECS[1].1,
        CILHelper::LoadWord     => &HELPER_SPECS[2].1,
        CILHelper::StoreWord    => &HELPER_SPECS[3].1,
        CILHelper::Syscall      => &HELPER_SPECS[4].1,
    }
}

// ===========================================================================
// CILBackendConfig
// ===========================================================================

/// Backend configuration for the CLR lowering pipeline.
#[derive(Debug, Clone)]
pub struct CILBackendConfig {
    /// Virtual register index used as the syscall argument (default: 4).
    pub syscall_arg_reg: usize,
    /// Maximum total static data in bytes (default: 16 MiB).
    pub max_static_data_bytes: usize,
    /// `maxstack` metadata for emitted CIL methods (default: 16).
    pub method_max_stack: u16,
    /// Number of argument registers passed to called functions (default: 0).
    pub call_register_count: usize,
}

impl Default for CILBackendConfig {
    fn default() -> Self {
        Self {
            syscall_arg_reg: 4,
            max_static_data_bytes: 16 * 1024 * 1024,
            method_max_stack: 16,
            call_register_count: 0,
        }
    }
}

// ===========================================================================
// CIL token provider
// ===========================================================================

/// Provides CLR metadata tokens for methods and helpers.
///
/// The CLR calls methods by 32-bit metadata token (encoded as a 4-byte
/// little-endian operand in `call` instructions).  Different PE-file
/// packagers may assign tokens differently, so we abstract that here.
pub trait CILTokenProvider: Send + Sync {
    /// Return the `MethodDef` token for an IR-callable function.
    fn method_token(&self, method_name: &str) -> u32;
    /// Return the helper's `MemberRef` / `MethodDef` token.
    fn helper_token(&self, helper: CILHelper) -> u32;
}

/// A deterministic token provider that assigns tokens sequentially.
///
/// - Method tokens: `0x06000001 + ordinal` (ordinal = position in callable list)
/// - Helper tokens: `0x0A000001 + ordinal` (ordinal = `CILHelper` discriminant order)
#[derive(Debug, Clone)]
pub struct SequentialCILTokenProvider {
    method_map: HashMap<String, u32>,
}

impl SequentialCILTokenProvider {
    /// Build a token map for the given list of callable labels.
    ///
    /// Panics if the number of callable labels overflows `u32` (which would
    /// require billions of methods — impossible in practice).
    pub fn new(callable_labels: &[&str]) -> Self {
        let mut method_map = HashMap::new();
        for (i, label) in callable_labels.iter().enumerate() {
            let token = 0x0600_0001u32.checked_add(
                u32::try_from(i).expect("callable label count overflows u32")
            ).expect("method token overflows u32");
            method_map.insert((*label).to_string(), token);
        }
        Self { method_map }
    }
}

impl CILTokenProvider for SequentialCILTokenProvider {
    fn method_token(&self, method_name: &str) -> u32 {
        *self.method_map.get(method_name).unwrap_or(&0x0600_0001)
    }

    fn helper_token(&self, helper: CILHelper) -> u32 {
        let idx = match helper {
            CILHelper::MemLoadByte  => 0,
            CILHelper::MemStoreByte => 1,
            CILHelper::LoadWord     => 2,
            CILHelper::StoreWord    => 3,
            CILHelper::Syscall      => 4,
        };
        0x0A00_0001 + idx
    }
}

// ===========================================================================
// Artifacts
// ===========================================================================

/// A compiled CIL method artifact — the body bytes plus metadata.
#[derive(Debug, Clone)]
pub struct CILMethodArtifact {
    /// The method's label / entry-point name.
    pub name: String,
    /// Assembled CIL bytecode (ready to wrap in a `.method` header).
    pub body: Vec<u8>,
    /// `maxstack` for the `.method` header.
    pub max_stack: u16,
    /// Local variable type strings, one per IR virtual register used.
    pub local_types: Vec<String>,
    /// Return type string (always `"int32"` in v1).
    pub return_type: &'static str,
    /// Parameter type strings for non-entry methods (empty for entry).
    pub parameter_types: Vec<String>,
}

impl CILMethodArtifact {
    /// Number of local variables declared for this method.
    pub fn local_count(&self) -> usize {
        self.local_types.len()
    }
}

/// A complete lowered program: one or more CIL methods + layout metadata.
pub struct CILProgramArtifact {
    /// The entry-point label.
    pub entry_label: String,
    /// Lowered methods, in callable order (entry first).
    pub methods: Vec<CILMethodArtifact>,
    /// Map from data-label name → byte offset within the flat data segment.
    pub data_offsets: HashMap<String, usize>,
    /// Total static data size in bytes.
    pub data_size: usize,
    /// Helper method specifications needed by the CLR runtime shim.
    pub helper_specs: Vec<&'static CILHelperSpec>,
    /// Token assignments for calls.
    pub token_provider: Box<dyn CILTokenProvider>,
}

impl CILProgramArtifact {
    /// Returns the entry method.
    pub fn entry_method(&self) -> Option<&CILMethodArtifact> {
        self.methods.first()
    }
}

// ===========================================================================
// Validation
// ===========================================================================

/// The set of IrOp variants supported by the CLR backend.
const CLR_SUPPORTED_OPS: &[IrOp] = &[
    IrOp::Label,
    IrOp::Comment,
    IrOp::Nop,
    IrOp::Halt,
    IrOp::Ret,
    IrOp::LoadImm,
    IrOp::LoadAddr,
    IrOp::LoadByte,
    IrOp::LoadWord,
    IrOp::StoreByte,
    IrOp::StoreWord,
    IrOp::Add,
    IrOp::AddImm,
    IrOp::Sub,
    IrOp::Mul,
    IrOp::Div,
    IrOp::And,
    IrOp::AndImm,
    IrOp::CmpEq,
    IrOp::CmpNe,
    IrOp::CmpLt,
    IrOp::CmpGt,
    IrOp::Jump,
    IrOp::BranchZ,
    IrOp::BranchNz,
    IrOp::Call,
    IrOp::Syscall,
];

/// Validate an `IrProgram` for the CLR target.
///
/// Returns a list of human-readable error strings.  An empty list means the
/// program is valid and can be lowered.
///
/// # Validation rules
///
/// 1. **Opcode support** — only the 25 opcodes listed in `CLR_SUPPORTED_OPS`
///    are accepted.
/// 2. **Immediate range** — `LOAD_IMM` and `ADD_IMM` immediates must fit in
///    a 32-bit signed integer (−2^31 .. 2^31−1).
/// 3. **SYSCALL numbers** — only 1 (write), 2 (read), 10 (exit) are valid.
/// 4. **Static data size** — sum of all data declaration sizes must not exceed
///    16 MiB (configurable via `CILBackendConfig`).
pub fn validate_for_clr(program: &IrProgram) -> Vec<String> {
    let mut errors = Vec::new();
    let supported: HashSet<IrOp> = CLR_SUPPORTED_OPS.iter().copied().collect();

    for instr in &program.instructions {
        let op = instr.opcode;

        // Rule 1: opcode support
        if !supported.contains(&op) {
            errors.push(format!(
                "opcode {:?} is not supported by the CLR backend",
                op
            ));
            continue;
        }

        // Rule 2: immediate range for LOAD_IMM and ADD_IMM
        if op == IrOp::LoadImm || op == IrOp::AddImm {
            for operand in &instr.operands {
                if let IrOperand::Immediate(v) = operand {
                    if *v < i32::MIN as i64 || *v > i32::MAX as i64 {
                        errors.push(format!(
                            "immediate {} is out of CLR int32 range [{}, {}]",
                            v, i32::MIN, i32::MAX
                        ));
                    }
                }
            }
        }

        // Rule 3: SYSCALL number whitelist
        if op == IrOp::Syscall {
            if let Some(IrOperand::Immediate(num)) = instr.operands.first() {
                if ![1, 2, 10].contains(num) {
                    errors.push(format!(
                        "SYSCALL number {} is not supported (allowed: 1=write, 2=read, 10=exit)",
                        num
                    ));
                }
            }
        }
    }

    // Rule 4: static data size
    let total: usize = program.data.iter().map(|d| d.size).sum();
    let max = CILBackendConfig::default().max_static_data_bytes;
    if total > max {
        errors.push(format!(
            "total static data {} bytes exceeds CLR limit {} bytes",
            total, max
        ));
    }

    errors
}

// ===========================================================================
// Internal analysis types
// ===========================================================================

/// A callable region: a contiguous slice of instructions that forms one CIL
/// method.  Every `CALL` target becomes a separate region.
#[derive(Debug, Clone)]
struct CallableRegion {
    /// The entry label for this region.
    label: String,
    /// Instruction indices (into `IrProgram::instructions`) that belong here.
    instr_indices: Vec<usize>,
    /// True for the program's entry region.
    is_entry: bool,
}

/// Shared analysis result consumed by all lowering passes.
struct LoweringPlan {
    /// Callable regions in the order they appear in the program.
    regions: Vec<CallableRegion>,
    /// data-label → byte offset within flat data segment.
    data_offsets: HashMap<String, usize>,
    /// Total size of the flat data segment.
    data_size: usize,
    /// Max virtual register index used across the entire program.
    local_count: usize,
    /// Token provider built from the region list.
    token_provider: SequentialCILTokenProvider,
}

// ===========================================================================
// Analysis pass
// ===========================================================================

fn analyse_program(program: &IrProgram) -> Result<LoweringPlan, CILBackendError> {
    // ── Collect all label positions ────────────────────────────────────────
    let mut label_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, instr) in program.instructions.iter().enumerate() {
        if instr.opcode == IrOp::Label {
            if let Some(IrOperand::Label(name)) = instr.operands.first() {
                label_to_idx.insert(name.as_str(), i);
            }
        }
    }

    // ── Discover callable regions ──────────────────────────────────────────
    // The entry region starts at instruction 0 (or the first LABEL).
    // Every CALL target starts a new region.
    let mut region_starts: Vec<(String, bool)> = Vec::new(); // (label, is_entry)
    let entry_label = program.entry_label.clone();
    region_starts.push((entry_label.clone(), true));

    for instr in &program.instructions {
        if instr.opcode == IrOp::Call {
            if let Some(IrOperand::Label(target)) = instr.operands.first() {
                if !region_starts.iter().any(|(l, _)| l == target) {
                    region_starts.push((target.clone(), false));
                }
            }
        }
    }

    // ── Partition instructions into regions ────────────────────────────────
    // A region spans from its start label up to (but not including) the next
    // region start label, or until the end of the program.
    let region_label_set: HashSet<&str> =
        region_starts.iter().map(|(l, _)| l.as_str()).collect();

    let mut regions: Vec<CallableRegion> = region_starts
        .iter()
        .map(|(label, is_entry)| CallableRegion {
            label: label.clone(),
            instr_indices: Vec::new(),
            is_entry: *is_entry,
        })
        .collect();

    // Figure out which region each instruction belongs to.
    let mut current_region_idx = 0usize;
    for (i, instr) in program.instructions.iter().enumerate() {
        if instr.opcode == IrOp::Label {
            if let Some(IrOperand::Label(name)) = instr.operands.first() {
                if region_label_set.contains(name.as_str()) {
                    if let Some(pos) = regions.iter().position(|r| r.label == *name) {
                        current_region_idx = pos;
                    }
                }
            }
        }
        regions[current_region_idx].instr_indices.push(i);
    }

    // ── Static data offsets ────────────────────────────────────────────────
    let mut data_offsets = HashMap::new();
    let mut offset = 0usize;
    for decl in &program.data {
        data_offsets.insert(decl.label.clone(), offset);
        offset += decl.size;
    }
    let data_size = offset;

    // ── Local count ────────────────────────────────────────────────────────
    let mut max_reg = 0usize;
    for instr in &program.instructions {
        for op in &instr.operands {
            if let IrOperand::Register(idx) = op {
                max_reg = max_reg.max(*idx);
            }
        }
    }
    let local_count = if program.instructions.is_empty() { 0 } else { max_reg + 1 };

    // ── Token provider ─────────────────────────────────────────────────────
    let callable_labels: Vec<&str> = regions.iter().map(|r| r.label.as_str()).collect();
    let token_provider = SequentialCILTokenProvider::new(&callable_labels);

    Ok(LoweringPlan {
        regions,
        data_offsets,
        data_size,
        local_count,
        token_provider,
    })
}

// ===========================================================================
// Lowering pass — one region → one CILMethodArtifact
// ===========================================================================

fn lower_region(
    program: &IrProgram,
    config: &CILBackendConfig,
    plan: &LoweringPlan,
    region: &CallableRegion,
) -> Result<CILMethodArtifact, CILBackendError> {
    let mut b = CILBytecodeBuilder::new();
    let tp = &plan.token_provider;
    let local_count = plan.local_count;

    // Non-entry methods receive their caller-saved argument registers as
    // CIL parameters (param indices 0..call_register_count).  Save them into
    // the corresponding locals so the instruction emitter can treat them
    // uniformly.
    let param_count = if region.is_entry {
        0
    } else {
        config.call_register_count.min(local_count)
    };
    for i in 0..param_count {
        b.emit_raw(crate::builder::encode_ldarg(i as u8));
        b.emit_raw(encode_stloc(i as u16));
    }

    // ── Emit instructions ──────────────────────────────────────────────────
    for &instr_idx in &region.instr_indices {
        let instr = &program.instructions[instr_idx];
        emit_instruction(&mut b, instr, program, config, plan, tp)?;
    }

    let body = b.assemble().map_err(|e| CILBackendError(e.0))?;

    let local_types: Vec<String> = (0..local_count).map(|_| "int32".to_string()).collect();
    let parameter_types: Vec<String> = (0..param_count).map(|_| "int32".to_string()).collect();

    Ok(CILMethodArtifact {
        name: region.label.clone(),
        body,
        max_stack: config.method_max_stack,
        local_types,
        return_type: "int32",
        parameter_types,
    })
}

// ===========================================================================
// Instruction emission
// ===========================================================================

/// Emit CIL bytecode for a single IR instruction.
fn emit_instruction(
    b: &mut CILBytecodeBuilder,
    instr: &IrInstruction,
    _program: &IrProgram,
    config: &CILBackendConfig,
    plan: &LoweringPlan,
    tp: &SequentialCILTokenProvider,
) -> Result<(), CILBackendError> {
    // Helpers to extract operands safely
    let reg = |i: usize| -> Option<usize> {
        instr.operands.get(i).and_then(|o| if let IrOperand::Register(r) = o { Some(*r) } else { None })
    };
    let imm = |i: usize| -> Option<i64> {
        instr.operands.get(i).and_then(|o| if let IrOperand::Immediate(v) = o { Some(*v) } else { None })
    };
    let lbl = |i: usize| -> Option<&str> {
        instr.operands.get(i).and_then(|o| if let IrOperand::Label(n) = o { Some(n.as_str()) } else { None })
    };

    match instr.opcode {
        // ── Meta / control flow ────────────────────────────────────────────

        IrOp::Label => {
            if let Some(name) = lbl(0) {
                b.mark(name);
            }
        }

        IrOp::Comment | IrOp::Nop => {
            b.emit_nop();
        }

        // ── LOAD_IMM  dst, imm ────────────────────────────────────────────

        IrOp::LoadImm => {
            let dst = reg(0).ok_or_else(|| CILBackendError("LOAD_IMM: missing dst".into()))?;
            let val = imm(1).ok_or_else(|| CILBackendError("LOAD_IMM: missing imm".into()))? as i32;
            b.emit_raw(encode_ldc_i4(val));
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── LOAD_ADDR  dst, label ─────────────────────────────────────────

        IrOp::LoadAddr => {
            let dst = reg(0).ok_or_else(|| CILBackendError("LOAD_ADDR: missing dst".into()))?;
            let name = lbl(1).ok_or_else(|| CILBackendError("LOAD_ADDR: missing label".into()))?;
            let offset = plan.data_offsets.get(name)
                .copied()
                .ok_or_else(|| CILBackendError(format!("LOAD_ADDR: unknown data label {name}")))?;
            let offset_i32 = i32::try_from(offset).map_err(|_| {
                CILBackendError(format!("LOAD_ADDR: data offset for '{name}' overflows i32"))
            })?;
            b.emit_raw(encode_ldc_i4(offset_i32));
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── LOAD_BYTE  dst, base, off ─────────────────────────────────────
        // CIL: ldloc base; ldloc off; add; call MemLoadByte; stloc dst

        IrOp::LoadByte => {
            let dst  = reg(0).ok_or_else(|| CILBackendError("LOAD_BYTE: missing dst".into()))?;
            let base = reg(1).ok_or_else(|| CILBackendError("LOAD_BYTE: missing base".into()))?;
            let off  = reg(2).ok_or_else(|| CILBackendError("LOAD_BYTE: missing off".into()))?;
            b.emit_raw(encode_ldloc(base as u16));
            b.emit_raw(encode_ldloc(off as u16));
            b.emit_add();
            b.emit_call(tp.helper_token(CILHelper::MemLoadByte));
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── LOAD_WORD  dst, base, off ─────────────────────────────────────
        // CIL: ldloc base; ldloc off; add; call LoadWord; stloc dst

        IrOp::LoadWord => {
            let dst  = reg(0).ok_or_else(|| CILBackendError("LOAD_WORD: missing dst".into()))?;
            let base = reg(1).ok_or_else(|| CILBackendError("LOAD_WORD: missing base".into()))?;
            let off  = reg(2).ok_or_else(|| CILBackendError("LOAD_WORD: missing off".into()))?;
            b.emit_raw(encode_ldloc(base as u16));
            b.emit_raw(encode_ldloc(off as u16));
            b.emit_add();
            b.emit_call(tp.helper_token(CILHelper::LoadWord));
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── STORE_BYTE  val, base, off ────────────────────────────────────
        // CIL: ldloc base; ldloc off; add; ldloc val; call MemStoreByte

        IrOp::StoreByte => {
            let val  = reg(0).ok_or_else(|| CILBackendError("STORE_BYTE: missing val".into()))?;
            let base = reg(1).ok_or_else(|| CILBackendError("STORE_BYTE: missing base".into()))?;
            let off  = reg(2).ok_or_else(|| CILBackendError("STORE_BYTE: missing off".into()))?;
            b.emit_raw(encode_ldloc(base as u16));
            b.emit_raw(encode_ldloc(off as u16));
            b.emit_add();
            b.emit_raw(encode_ldloc(val as u16));
            b.emit_call(tp.helper_token(CILHelper::MemStoreByte));
        }

        // ── STORE_WORD  val, base, off ────────────────────────────────────
        // CIL: ldloc base; ldloc off; add; ldloc val; call StoreWord

        IrOp::StoreWord => {
            let val  = reg(0).ok_or_else(|| CILBackendError("STORE_WORD: missing val".into()))?;
            let base = reg(1).ok_or_else(|| CILBackendError("STORE_WORD: missing base".into()))?;
            let off  = reg(2).ok_or_else(|| CILBackendError("STORE_WORD: missing off".into()))?;
            b.emit_raw(encode_ldloc(base as u16));
            b.emit_raw(encode_ldloc(off as u16));
            b.emit_add();
            b.emit_raw(encode_ldloc(val as u16));
            b.emit_call(tp.helper_token(CILHelper::StoreWord));
        }

        // ── ADD  dst, lhs, rhs ────────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; add; stloc dst

        IrOp::Add => {
            let dst = reg(0).ok_or_else(|| CILBackendError("ADD: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("ADD: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("ADD: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_add();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── ADD_IMM  dst, src, imm ────────────────────────────────────────
        // CIL: ldloc src; ldc.i4 imm; add; stloc dst

        IrOp::AddImm => {
            let dst = reg(0).ok_or_else(|| CILBackendError("ADD_IMM: missing dst".into()))?;
            let src = reg(1).ok_or_else(|| CILBackendError("ADD_IMM: missing src".into()))?;
            let val = imm(2).ok_or_else(|| CILBackendError("ADD_IMM: missing imm".into()))? as i32;
            b.emit_raw(encode_ldloc(src as u16));
            b.emit_raw(encode_ldc_i4(val));
            b.emit_add();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── SUB  dst, lhs, rhs ────────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; sub; stloc dst

        IrOp::Sub => {
            let dst = reg(0).ok_or_else(|| CILBackendError("SUB: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("SUB: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("SUB: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_sub();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── MUL  dst, lhs, rhs ────────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; mul; stloc dst

        IrOp::Mul => {
            let dst = reg(0).ok_or_else(|| CILBackendError("MUL: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("MUL: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("MUL: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_mul();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── DIV  dst, lhs, rhs ────────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; div; stloc dst
        // CIL `div` truncates toward zero — matches IrOp::Div semantics.

        IrOp::Div => {
            let dst = reg(0).ok_or_else(|| CILBackendError("DIV: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("DIV: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("DIV: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_div();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── AND  dst, lhs, rhs ────────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; and; stloc dst

        IrOp::And => {
            let dst = reg(0).ok_or_else(|| CILBackendError("AND: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("AND: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("AND: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_and();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── AND_IMM  dst, src, imm ────────────────────────────────────────
        // CIL: ldloc src; ldc.i4 imm; and; stloc dst

        IrOp::AndImm => {
            let dst = reg(0).ok_or_else(|| CILBackendError("AND_IMM: missing dst".into()))?;
            let src = reg(1).ok_or_else(|| CILBackendError("AND_IMM: missing src".into()))?;
            let val = imm(2).ok_or_else(|| CILBackendError("AND_IMM: missing imm".into()))? as i32;
            b.emit_raw(encode_ldloc(src as u16));
            b.emit_raw(encode_ldc_i4(val));
            b.emit_and();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── CMP_EQ  dst, lhs, rhs ────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; ceq; stloc dst

        IrOp::CmpEq => {
            let dst = reg(0).ok_or_else(|| CILBackendError("CMP_EQ: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("CMP_EQ: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("CMP_EQ: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_ceq();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── CMP_NE  dst, lhs, rhs ────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; ceq; ldc.i4.0; ceq; stloc dst
        // (double-invert: a != b  →  NOT (a == b))

        IrOp::CmpNe => {
            let dst = reg(0).ok_or_else(|| CILBackendError("CMP_NE: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("CMP_NE: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("CMP_NE: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_ceq();
            b.emit_raw(encode_ldc_i4(0));
            b.emit_ceq(); // NOT
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── CMP_LT  dst, lhs, rhs ────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; clt; stloc dst

        IrOp::CmpLt => {
            let dst = reg(0).ok_or_else(|| CILBackendError("CMP_LT: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("CMP_LT: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("CMP_LT: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_clt();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── CMP_GT  dst, lhs, rhs ────────────────────────────────────────
        // CIL: ldloc lhs; ldloc rhs; cgt; stloc dst

        IrOp::CmpGt => {
            let dst = reg(0).ok_or_else(|| CILBackendError("CMP_GT: missing dst".into()))?;
            let lhs = reg(1).ok_or_else(|| CILBackendError("CMP_GT: missing lhs".into()))?;
            let rhs = reg(2).ok_or_else(|| CILBackendError("CMP_GT: missing rhs".into()))?;
            b.emit_raw(encode_ldloc(lhs as u16));
            b.emit_raw(encode_ldloc(rhs as u16));
            b.emit_cgt();
            b.emit_raw(encode_stloc(dst as u16));
        }

        // ── JUMP  target ──────────────────────────────────────────────────

        IrOp::Jump => {
            let target = lbl(0).ok_or_else(|| CILBackendError("JUMP: missing label".into()))?;
            b.emit_branch(CILBranchKind::Always, target, false);
        }

        // ── BRANCH_Z  cond, target ────────────────────────────────────────
        // Jump to target if cond == 0 (false)

        IrOp::BranchZ => {
            let cond   = reg(0).ok_or_else(|| CILBackendError("BRANCH_Z: missing cond".into()))?;
            let target = lbl(1).ok_or_else(|| CILBackendError("BRANCH_Z: missing label".into()))?;
            b.emit_raw(encode_ldloc(cond as u16));
            b.emit_branch(CILBranchKind::False, target, false);
        }

        // ── BRANCH_NZ  cond, target ───────────────────────────────────────
        // Jump to target if cond != 0 (true)

        IrOp::BranchNz => {
            let cond   = reg(0).ok_or_else(|| CILBackendError("BRANCH_NZ: missing cond".into()))?;
            let target = lbl(1).ok_or_else(|| CILBackendError("BRANCH_NZ: missing label".into()))?;
            b.emit_raw(encode_ldloc(cond as u16));
            b.emit_branch(CILBranchKind::True, target, false);
        }

        // ── CALL  target ──────────────────────────────────────────────────
        // Push argument registers v0..call_register_count, call method,
        // store return value into v1.

        IrOp::Call => {
            let target = lbl(0).ok_or_else(|| CILBackendError("CALL: missing label".into()))?;
            let n = config.call_register_count.min(plan.local_count);
            for i in 0..n {
                b.emit_raw(encode_ldloc(i as u16));
            }
            b.emit_call(tp.method_token(target));
            // Store return value into v1 (conventional return register)
            if plan.local_count > 1 {
                b.emit_raw(encode_stloc(1u16));
            } else {
                b.emit_pop();
            }
        }

        // ── RET ───────────────────────────────────────────────────────────
        // Load the return value from v1 and return.

        IrOp::Ret => {
            if plan.local_count > 1 {
                b.emit_raw(encode_ldloc(1u16));
            } else {
                b.emit_raw(encode_ldc_i4(0));
            }
            b.emit_ret();
        }

        // ── HALT ──────────────────────────────────────────────────────────
        // Same as RET in the v1 CLR model: load v1 and return.

        IrOp::Halt => {
            if plan.local_count > 1 {
                b.emit_raw(encode_ldloc(1u16));
            } else {
                b.emit_raw(encode_ldc_i4(0));
            }
            b.emit_ret();
        }

        // ── SYSCALL  num, arg_reg ─────────────────────────────────────────
        // CIL: ldc.i4 num; ldloc syscall_arg_reg; call Syscall; stloc 1

        IrOp::Syscall => {
            let num = imm(0).ok_or_else(|| CILBackendError("SYSCALL: missing number".into()))? as i32;
            let arg_reg = config.syscall_arg_reg.min(plan.local_count.saturating_sub(1));
            b.emit_raw(encode_ldc_i4(num));
            b.emit_raw(encode_ldloc(arg_reg as u16));
            b.emit_call(tp.helper_token(CILHelper::Syscall));
            if plan.local_count > 1 {
                b.emit_raw(encode_stloc(1u16));
            } else {
                b.emit_pop();
            }
        }

    }

    Ok(())
}

// ===========================================================================
// Public entry point
// ===========================================================================

/// Lower an `IrProgram` to a `CILProgramArtifact`.
///
/// Runs `validate_for_clr()` internally; returns an error if validation fails.
///
/// # Errors
///
/// Returns `CILBackendError` if validation fails or lowering encounters an
/// internal inconsistency.
pub fn lower_ir_to_cil_bytecode(
    program: &IrProgram,
    config: Option<CILBackendConfig>,
    _token_provider: Option<Box<dyn CILTokenProvider>>,
) -> Result<CILProgramArtifact, CILBackendError> {
    // Pre-flight validation
    let errors = validate_for_clr(program);
    if !errors.is_empty() {
        return Err(CILBackendError(errors.join("; ")));
    }

    let config = config.unwrap_or_default();
    let plan = analyse_program(program)?;

    // Lower each callable region
    let mut methods = Vec::new();
    for region in &plan.regions {
        let artifact = lower_region(program, &config, &plan, region)?;
        methods.push(artifact);
    }

    // Collect helper specs for all five helpers
    let helper_specs: Vec<&'static CILHelperSpec> =
        HELPER_SPECS.iter().map(|(_, spec)| spec).collect();

    let callable_labels: Vec<&str> = plan.regions.iter().map(|r| r.label.as_str()).collect();
    let token_provider = Box::new(SequentialCILTokenProvider::new(&callable_labels));

    Ok(CILProgramArtifact {
        entry_label: program.entry_label.clone(),
        methods,
        data_offsets: plan.data_offsets,
        data_size: plan.data_size,
        helper_specs,
        token_provider,
    })
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use compiler_ir::{IrInstruction, IrOp, IrOperand, IrProgram};

    fn minimal_prog() -> IrProgram {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(42)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        prog
    }

    // ------------------------------------------------------------------
    // validate_for_clr
    // ------------------------------------------------------------------

    #[test]
    fn test_validate_valid_program_returns_empty() {
        let prog = minimal_prog();
        assert!(validate_for_clr(&prog).is_empty());
    }

    #[test]
    fn test_validate_rejects_out_of_range_immediate() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(i64::MAX)],
            1,
        ));
        let errors = validate_for_clr(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("out of CLR int32 range"));
    }

    #[test]
    fn test_validate_rejects_invalid_syscall() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            1,
        ));
        let errors = validate_for_clr(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("SYSCALL number 99"));
    }

    #[test]
    fn test_validate_allows_syscall_1_2_10() {
        for num in [1i64, 2, 10] {
            let mut prog = IrProgram::new("_start");
            prog.add_instruction(IrInstruction::new(
                IrOp::Syscall,
                vec![IrOperand::Immediate(num)],
                1,
            ));
            prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
            let errors = validate_for_clr(&prog);
            let syscall_errors: Vec<_> = errors.iter().filter(|e| e.contains("SYSCALL number")).collect();
            assert!(syscall_errors.is_empty(), "syscall {} should be allowed", num);
        }
    }

    #[test]
    fn test_validate_rejects_excess_static_data() {
        use compiler_ir::IrDataDecl;
        let mut prog = IrProgram::new("_start");
        // 17 MiB > 16 MiB limit
        prog.data.push(IrDataDecl { label: "huge".to_string(), size: 17 * 1024 * 1024, init: 0 });
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let errors = validate_for_clr(&prog);
        assert!(!errors.is_empty());
        assert!(errors[0].contains("exceeds CLR limit"));
    }

    // ------------------------------------------------------------------
    // lower_ir_to_cil_bytecode
    // ------------------------------------------------------------------

    #[test]
    fn test_lower_minimal_program() {
        let prog = minimal_prog();
        let result = lower_ir_to_cil_bytecode(&prog, None, None);
        assert!(result.is_ok(), "lowering failed: {:?}", result.err());
        let artifact = result.unwrap();
        assert_eq!(artifact.entry_label, "_start");
        assert!(!artifact.methods.is_empty());
    }

    #[test]
    fn test_lower_produces_non_empty_body() {
        let prog = minimal_prog();
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        let body = &artifact.methods[0].body;
        assert!(!body.is_empty());
    }

    #[test]
    fn test_lower_load_imm_stores_to_local() {
        let prog = minimal_prog();
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        let body = &artifact.methods[0].body;
        // ldc.i4.s 42 → [0x1F, 42]; stloc.1 → [0x0B]
        assert!(body.windows(2).any(|w| w == [0x1F, 42]));
    }

    #[test]
    fn test_lower_halt_emits_ret() {
        let prog = minimal_prog();
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        let body = &artifact.methods[0].body;
        assert!(body.contains(&0x2A), "expected ret (0x2A) in: {body:?}");
    }

    #[test]
    fn test_lower_data_offsets_populated() {
        use compiler_ir::IrDataDecl;
        let mut prog = IrProgram::new("_start");
        prog.data.push(IrDataDecl { label: "tape".to_string(), size: 100, init: 0 });
        prog.data.push(IrDataDecl { label: "buf".to_string(), size: 50, init: 0 });
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 1));
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        assert_eq!(artifact.data_offsets["tape"], 0);
        assert_eq!(artifact.data_offsets["buf"], 100);
        assert_eq!(artifact.data_size, 150);
    }

    #[test]
    fn test_lower_add_instruction() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(1), IrOperand::Immediate(3)],
            1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(2), IrOperand::Immediate(4)],
            2,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Add,
            vec![
                IrOperand::Register(1),
                IrOperand::Register(1),
                IrOperand::Register(2),
            ],
            3,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 4));
        let result = lower_ir_to_cil_bytecode(&prog, None, None);
        assert!(result.is_ok(), "{:?}", result.err());
        let body = &result.unwrap().methods[0].body;
        // add instruction = 0x58
        assert!(body.contains(&0x58), "expected add (0x58) in body");
    }

    #[test]
    fn test_lower_cmp_eq() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::CmpEq,
            vec![
                IrOperand::Register(0),
                IrOperand::Register(1),
                IrOperand::Register(2),
            ],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        let body = &artifact.methods[0].body;
        // ceq = 0xFE 0x01
        assert!(
            body.windows(2).any(|w| w == [0xFE, 0x01]),
            "expected ceq (0xFE 0x01) in body"
        );
    }

    #[test]
    fn test_lower_cmp_ne_double_inverts() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::CmpNe,
            vec![
                IrOperand::Register(0),
                IrOperand::Register(1),
                IrOperand::Register(2),
            ],
            1,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 2));
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        let body = &artifact.methods[0].body;
        // two ceq opcodes = two [0xFE, 0x01] sequences
        let count = body.windows(2).filter(|w| *w == [0xFE, 0x01]).count();
        assert_eq!(count, 2, "expected two ceq sequences for CMP_NE");
    }

    #[test]
    fn test_lower_branch_nz() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("loop".to_string())],
            1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::LoadImm,
            vec![IrOperand::Register(0), IrOperand::Immediate(1)],
            2,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::BranchNz,
            vec![
                IrOperand::Register(0),
                IrOperand::Label("loop".to_string()),
            ],
            3,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 4));
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        let body = &artifact.methods[0].body;
        // brtrue.s = 0x2D, brtrue = 0x3A
        assert!(
            body.contains(&0x2D) || body.contains(&0x3A),
            "expected brtrue in: {body:?}"
        );
    }

    #[test]
    fn test_lower_jump() {
        let mut prog = IrProgram::new("_start");
        prog.add_instruction(IrInstruction::new(
            IrOp::Jump,
            vec![IrOperand::Label("end".to_string())],
            1,
        ));
        prog.add_instruction(IrInstruction::new(
            IrOp::Label,
            vec![IrOperand::Label("end".to_string())],
            2,
        ));
        prog.add_instruction(IrInstruction::new(IrOp::Halt, vec![], 3));
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        let body = &artifact.methods[0].body;
        // br.s = 0x2B
        assert!(body.contains(&0x2B), "expected br.s in: {body:?}");
    }

    #[test]
    fn test_lower_invalid_program_returns_error() {
        let mut prog = IrProgram::new("_start");
        // Use an unsupported opcode
        prog.add_instruction(IrInstruction::new(
            IrOp::Syscall,
            vec![IrOperand::Immediate(99)],
            1,
        ));
        let result = lower_ir_to_cil_bytecode(&prog, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_lower_helper_specs_present() {
        let prog = minimal_prog();
        let artifact = lower_ir_to_cil_bytecode(&prog, None, None).unwrap();
        assert_eq!(artifact.helper_specs.len(), 5);
        assert_eq!(artifact.helper_specs[0].name, "MemLoadByte");
    }

    #[test]
    fn test_token_provider_method_token() {
        let tp = SequentialCILTokenProvider::new(&["_start", "fn1", "fn2"]);
        assert_eq!(tp.method_token("_start"), 0x0600_0001);
        assert_eq!(tp.method_token("fn1"),    0x0600_0002);
        assert_eq!(tp.method_token("fn2"),    0x0600_0003);
    }

    #[test]
    fn test_token_provider_helper_token() {
        let tp = SequentialCILTokenProvider::new(&["_start"]);
        assert_eq!(tp.helper_token(CILHelper::MemLoadByte),  0x0A00_0001);
        assert_eq!(tp.helper_token(CILHelper::MemStoreByte), 0x0A00_0002);
        assert_eq!(tp.helper_token(CILHelper::Syscall),      0x0A00_0005);
    }
}
