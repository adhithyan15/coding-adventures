//! Symbolic execution of WIR and bit-precise proofs (VER00 §7.6.2, VER19, VER-P024).

use std::collections::HashMap;

use bit_blaster::aig::{Aig, Edge, Node};
use bit_blaster::bv::{self, BitVec};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::{Cnf, SolveResult, Solver};

use crate::ast::{
    BinaryOp, BlockId, Function, ProvenanceId, UnaryOp, VarId, WirExpr, WirInst, WirTerminator,
    WirType,
};

/// Extract concrete `u128` integer from a `BitVec` if all its bits are constant.
pub fn bitvec_concrete_u128(bv: &BitVec) -> Option<u128> {
    let mut val = 0u128;
    for (i, edge) in bv.bits.iter().enumerate() {
        if !edge.is_constant() {
            return None;
        }
        if *edge == Edge::TRUE {
            if i < 128 {
                val |= 1 << i;
            } else {
                return None;
            }
        }
    }
    Some(val)
}

/// Zero-extend `bv` to `new_width` bits.
pub fn zero_extend(bv: &BitVec, new_width: usize) -> BitVec {
    let mut bits = bv.bits.clone();
    if new_width > bits.len() {
        bits.resize(new_width, Edge::FALSE);
    }
    BitVec { bits }
}

/// Sign-extend `bv` to `new_width` bits.
pub fn sign_extend(bv: &BitVec, new_width: usize) -> BitVec {
    let mut bits = bv.bits.clone();
    let msb = bv.msb();
    if new_width > bits.len() {
        bits.resize(new_width, msb);
    }
    BitVec { bits }
}

/// Truncate `bv` to `new_width` bits.
pub fn truncate(bv: &BitVec, new_width: usize) -> BitVec {
    let w = new_width.min(bv.bits.len());
    BitVec {
        bits: bv.bits[..w].to_vec(),
    }
}

/// Concatenate two `BitVec`s: `low` followed by `high`.
pub fn concat(low: &BitVec, high: &BitVec) -> BitVec {
    let mut bits = low.bits.clone();
    bits.extend(high.bits.iter().copied());
    BitVec { bits }
}

/// A symbolic value tracked during symbolic execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymVal {
    /// Fixed-width bit-vector (scalar integers, booleans encoded as 1 bit).
    Bits(BitVec),
    /// Boolean condition edge in the AIG.
    Bool(Edge),
    /// Pointer with provenance and 64-bit byte offset.
    Ptr(ProvenanceId, BitVec),
    /// Tuple of symbolic values.
    Tuple(Vec<SymVal>),
    /// Unit value.
    Unit,
}

impl SymVal {
    /// Create a constant bit-vector value.
    pub fn constant_bits(width: usize, val: u128) -> Self {
        Self::Bits(BitVec::constant(width, val))
    }

    /// Create a constant boolean value.
    pub fn constant_bool(val: bool) -> Self {
        Self::Bool(Edge::constant(val))
    }

    /// Get the bit width if this is a bit-vector.
    pub fn width(&self) -> Option<usize> {
        match self {
            Self::Bits(b) => Some(b.width()),
            Self::Bool(_) => Some(1),
            Self::Ptr(_, offset) => Some(offset.width()),
            _ => None,
        }
    }

    /// Convert or coerce to a `BitVec`.
    pub fn to_bits(&self) -> Result<BitVec, SymbolicError> {
        match self {
            Self::Bits(b) => Ok(b.clone()),
            Self::Bool(e) => Ok(BitVec { bits: vec![*e] }),
            Self::Ptr(_, offset) => Ok(offset.clone()),
            _ => Err(SymbolicError::TypeMismatch(
                "Cannot convert non-scalar SymVal to BitVec".to_string(),
            )),
        }
    }

    /// Convert or coerce to a boolean `Edge`.
    pub fn to_bool(&self, aig: &mut Aig) -> Result<Edge, SymbolicError> {
        match self {
            Self::Bool(e) => Ok(*e),
            Self::Bits(b) => {
                if b.width() == 1 {
                    Ok(b.bits[0])
                } else {
                    let zero = BitVec::constant(b.width(), 0);
                    Ok(!bv::eq(aig, b, &zero))
                }
            }
            _ => Err(SymbolicError::TypeMismatch(
                "Cannot convert non-boolean SymVal to boolean Edge".to_string(),
            )),
        }
    }
}

/// An allocation in symbolic memory.
#[derive(Debug, Clone)]
pub struct SymAlloc {
    /// Base provenance identifier.
    pub base: ProvenanceId,
    /// Size of the allocation in bytes.
    pub size: usize,
    /// Stored byte contents as 8-bit `BitVec`s.
    pub bytes: Vec<BitVec>,
}

/// Symbolic memory tracking allocations and loads/stores.
#[derive(Debug, Clone, Default)]
pub struct SymMemory {
    /// Tracked provenance allocations.
    pub allocations: HashMap<ProvenanceId, SymAlloc>,
    /// Counter for allocating fresh provenance IDs.
    pub next_prov: u64,
}

impl SymMemory {
    /// Create an empty symbolic memory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a buffer of `size` bytes initialized to zeroes (or a specified constant byte).
    pub fn allocate(&mut self, size: usize, initial_byte: Option<u8>) -> (ProvenanceId, BitVec) {
        self.next_prov += 1;
        let prov = ProvenanceId(self.next_prov);
        let byte_val = initial_byte.unwrap_or(0) as u128;
        let bytes = (0..size)
            .map(|_| BitVec::constant(8, byte_val))
            .collect();
        self.allocations.insert(
            prov,
            SymAlloc {
                base: prov,
                size,
                bytes,
            },
        );
        (prov, BitVec::constant(64, 0))
    }

    /// Allocate a buffer of `size` symbolic byte inputs.
    pub fn allocate_symbolic(&mut self, size: usize, aig: &mut Aig) -> (ProvenanceId, BitVec) {
        self.next_prov += 1;
        let prov = ProvenanceId(self.next_prov);
        let bytes = (0..size).map(|_| BitVec::input(aig, 8)).collect();
        self.allocations.insert(
            prov,
            SymAlloc {
                base: prov,
                size,
                bytes,
            },
        );
        (prov, BitVec::constant(64, 0))
    }

    /// Load a value of type `ty` from pointer `(prov, offset)`.
    pub fn load(
        &self,
        prov: ProvenanceId,
        offset: &BitVec,
        ty: &WirType,
        aig: &mut Aig,
    ) -> Result<SymVal, SymbolicError> {
        let alloc = self.allocations.get(&prov).ok_or_else(|| {
            SymbolicError::TypeMismatch(format!("Invalid provenance {:?} in load", prov))
        })?;

        let byte_count = ty.byte_size().unwrap_or(1);
        if byte_count == 0 {
            return Ok(SymVal::Bits(BitVec::constant(0, 0)));
        }

        if let Some(concrete_off) = bitvec_concrete_u128(offset) {
            let start = usize::try_from(concrete_off).map_err(|_| SymbolicError::OutOfBounds {
                provenance: prov,
                offset: usize::MAX,
                size: byte_count,
                bound: alloc.size,
            })?;
            let end = start.checked_add(byte_count).ok_or(SymbolicError::OutOfBounds {
                provenance: prov,
                offset: start,
                size: byte_count,
                bound: alloc.size,
            })?;
            if end > alloc.size {
                return Err(SymbolicError::OutOfBounds {
                    provenance: prov,
                    offset: start,
                    size: byte_count,
                    bound: alloc.size,
                });
            }

            if byte_count == 1 {
                return Ok(SymVal::Bits(alloc.bytes[start].clone()));
            }

            let mut combined = alloc.bytes[start].clone();
            for i in 1..byte_count {
                combined = concat(&combined, &alloc.bytes[start + i]);
            }
            return Ok(SymVal::Bits(combined));
        }

        // Symbolic offset: construct a mux selection over all valid offsets
        if byte_count > alloc.size {
            return Err(SymbolicError::OutOfBounds {
                provenance: prov,
                offset: 0,
                size: byte_count,
                bound: alloc.size,
            });
        }

        let max_start = alloc.size - byte_count;
        let mut result = BitVec::constant(byte_count * 8, 0);
        for start in (0..=max_start).rev() {
            let mut candidate = alloc.bytes[start].clone();
            for i in 1..byte_count {
                candidate = concat(&candidate, &alloc.bytes[start + i]);
            }
            let is_match = bv::eq(aig, offset, &BitVec::constant(64, start as u128));
            result = bv::ite(aig, is_match, &candidate, &result);
        }

        Ok(SymVal::Bits(result))
    }

    /// Store a value of type `ty` to pointer `(prov, offset)`.
    pub fn store(
        &mut self,
        prov: ProvenanceId,
        offset: &BitVec,
        val: &SymVal,
        aig: &mut Aig,
    ) -> Result<(), SymbolicError> {
        let alloc = self.allocations.get_mut(&prov).ok_or_else(|| {
            SymbolicError::TypeMismatch(format!("Invalid provenance {:?} in store", prov))
        })?;

        let val_bits = val.to_bits()?;
        let byte_count = val_bits.width().div_ceil(8);
        if byte_count == 0 {
            return Ok(());
        }

        if let Some(concrete_off) = bitvec_concrete_u128(offset) {
            let start = usize::try_from(concrete_off).map_err(|_| SymbolicError::OutOfBounds {
                provenance: prov,
                offset: usize::MAX,
                size: byte_count,
                bound: alloc.size,
            })?;
            let end = start.checked_add(byte_count).ok_or(SymbolicError::OutOfBounds {
                provenance: prov,
                offset: start,
                size: byte_count,
                bound: alloc.size,
            })?;
            if end > alloc.size {
                return Err(SymbolicError::OutOfBounds {
                    provenance: prov,
                    offset: start,
                    size: byte_count,
                    bound: alloc.size,
                });
            }

            for i in 0..byte_count {
                let low = i * 8;
                let high = std::cmp::min((i + 1) * 8, val_bits.width());
                let byte_slice = BitVec {
                    bits: val_bits.bits[low..high].to_vec(),
                };
                let padded = zero_extend(&byte_slice, 8);
                alloc.bytes[start + i] = padded;
            }
            return Ok(());
        }

        // Symbolic offset: multiplex over all slots
        if byte_count > alloc.size {
            return Err(SymbolicError::OutOfBounds {
                provenance: prov,
                offset: 0,
                size: byte_count,
                bound: alloc.size,
            });
        }

        let max_start = alloc.size - byte_count;
        for i in 0..byte_count {
            let low = i * 8;
            let high = std::cmp::min((i + 1) * 8, val_bits.width());
            let byte_slice = BitVec {
                bits: val_bits.bits[low..high].to_vec(),
            };
            let byte_to_write = zero_extend(&byte_slice, 8);

            for start in 0..=max_start {
                let target_slot = start + i;
                let is_match = bv::eq(aig, offset, &BitVec::constant(64, start as u128));
                let old_byte = alloc.bytes[target_slot].clone();
                alloc.bytes[target_slot] = bv::ite(aig, is_match, &byte_to_write, &old_byte);
            }
        }

        Ok(())
    }
}

/// Errors occurring during symbolic execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolicError {
    /// Undefined variable in environment.
    UndefinedVariable(VarId),
    /// Type mismatch or invalid operand.
    TypeMismatch(String),
    /// Out-of-bounds memory access (CWE-125 / CWE-787).
    OutOfBounds {
        provenance: ProvenanceId,
        offset: usize,
        size: usize,
        bound: usize,
    },
    /// Step limit exceeded on execution path.
    StepLimitExceeded(usize),
    /// Maximum path exploration budget exceeded.
    PathBudgetExceeded(usize),
    /// Explicit trap instruction encountered.
    Trap(String),
    /// Allocation memory limit exceeded.
    MemoryLimitExceeded { requested: usize, max: usize },
    /// Unsupported operation in symbolic mode.
    UnsupportedOperation(String),
}

impl std::fmt::Display for SymbolicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndefinedVariable(v) => write!(f, "Undefined variable {:?}", v.0),
            Self::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            Self::OutOfBounds { provenance, offset, size, bound } => write!(
                f,
                "Out-of-bounds access at prov {:?}: offset {} + size {} > bound {}",
                provenance, offset, size, bound
            ),
            Self::StepLimitExceeded(limit) => write!(f, "Step limit exceeded: {}", limit),
            Self::PathBudgetExceeded(limit) => write!(f, "Path budget exceeded: {}", limit),
            Self::Trap(msg) => write!(f, "Execution trap: {}", msg),
            Self::MemoryLimitExceeded { requested, max } => write!(
                f,
                "Memory limit exceeded: requested {} bytes, max allowed is {}",
                requested, max
            ),
            Self::UnsupportedOperation(msg) => write!(f, "Unsupported symbolic operation: {}", msg),
        }
    }
}

impl std::error::Error for SymbolicError {}

/// Observable trace event recorded during symbolic execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservableSymEvent {
    /// Conditional branch decision.
    Branch { cond: Edge, taken: bool },
    /// Memory load operation.
    Load {
        provenance: ProvenanceId,
        offset: BitVec,
        size: usize,
    },
    /// Memory store operation.
    Store {
        provenance: ProvenanceId,
        offset: BitVec,
        size: usize,
    },
}

/// Outcome of exploring an individual symbolic execution path.
#[derive(Debug, Clone)]
pub struct PathOutcome {
    /// Accumulated path conditions along this path.
    pub path_condition: Vec<Edge>,
    /// Return value if the path terminated via `WirTerminator::Return`.
    pub return_val: Option<SymVal>,
    /// Final symbolic memory state.
    pub memory: SymMemory,
    /// Recorded observable trace events along this path.
    pub observable_events: Vec<ObservableSymEvent>,
    /// Whether this path was truncated by bounded loop unrolling.
    pub bounded: bool,
    /// Total instruction steps executed along this path.
    pub steps: usize,
}

/// Configuration parameters for symbolic execution.
#[derive(Debug, Clone)]
pub struct SymbolicConfig {
    /// Maximum instruction steps permitted per path.
    pub max_steps: usize,
    /// Maximum number of paths to explore.
    pub max_paths: usize,
    /// Loop unrolling bound `k`.
    pub unroll_bound: usize,
    /// Maximum total memory bytes that can be allocated.
    pub max_memory_bytes: usize,
    /// Optional conflict budget for the SAT solver (CWE-400 mitigation).
    pub solver_conflict_budget: Option<u64>,
}

impl Default for SymbolicConfig {
    fn default() -> Self {
        Self {
            max_steps: 100_000,
            max_paths: 256,
            unroll_bound: 16,
            max_memory_bytes: 64 * 1024 * 1024,
            solver_conflict_budget: Some(200_000),
        }
    }
}

/// Internal path state during worklist exploration.
#[derive(Clone)]
struct PathState {
    block_id: BlockId,
    env: HashMap<VarId, SymVal>,
    memory: SymMemory,
    path_conditions: Vec<Edge>,
    loop_counts: HashMap<BlockId, usize>,
    step_count: usize,
    observable_events: Vec<ObservableSymEvent>,
    bounded: bool,
}

/// Symbolic execution engine.
pub struct SymbolicExecutor {
    /// Engine configuration.
    pub config: SymbolicConfig,
    /// Underlying And-Inverter Graph for circuit generation.
    pub aig: Aig,
    /// Named input variables registered for counterexample extraction.
    pub input_vars: Vec<(String, BitVec)>,
}

impl Default for SymbolicExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolicExecutor {
    /// Create a new symbolic executor with default configuration.
    pub fn new() -> Self {
        Self::with_config(SymbolicConfig::default())
    }

    /// Create a new symbolic executor with specific configuration.
    pub fn with_config(config: SymbolicConfig) -> Self {
        Self {
            config,
            aig: Aig::new(),
            input_vars: Vec::new(),
        }
    }

    /// Access the underlying AIG.
    pub fn aig(&self) -> &Aig {
        &self.aig
    }

    /// Mutable access to the underlying AIG.
    pub fn aig_mut(&mut self) -> &mut Aig {
        &mut self.aig
    }

    /// Register a symbolic bit-vector input with a given name.
    pub fn create_input_bits(&mut self, name: &str, width: usize) -> BitVec {
        let bv = BitVec::input(&mut self.aig, width);
        self.input_vars.push((name.to_string(), bv.clone()));
        bv
    }

    /// Register a symbolic boolean input.
    pub fn create_input_bool(&mut self, name: &str) -> Edge {
        let e = self.aig.input();
        let bv = BitVec { bits: vec![e] };
        self.input_vars.push((name.to_string(), bv));
        e
    }

    /// Execute a WIR function symbolically.
    pub fn execute_function(
        &mut self,
        func: &Function,
        args: Vec<SymVal>,
        initial_memory: SymMemory,
    ) -> Result<Vec<PathOutcome>, SymbolicError> {
        let mut initial_env = HashMap::new();
        for ((var_id, _ty, _label), arg_val) in func.params.iter().zip(args) {
            initial_env.insert(var_id.clone(), arg_val);
        }

        let initial_block_id = func
            .blocks
            .first()
            .map(|b| b.id)
            .unwrap_or(BlockId(0));

        let initial_state = PathState {
            block_id: initial_block_id,
            env: initial_env,
            memory: initial_memory,
            path_conditions: Vec::new(),
            loop_counts: HashMap::new(),
            step_count: 0,
            observable_events: Vec::new(),
            bounded: false,
        };

        let mut worklist = vec![initial_state];
        let mut completed_paths = Vec::new();

        while let Some(mut state) = worklist.pop() {
            if completed_paths.len() + worklist.len() > self.config.max_paths {
                return Err(SymbolicError::PathBudgetExceeded(self.config.max_paths));
            }

            let block = func
                .blocks
                .iter()
                .find(|b| b.id == state.block_id)
                .ok_or_else(|| {
                    SymbolicError::TypeMismatch(format!("Block {:?} not found", state.block_id))
                })?;

            // Track loop visits
            let visit_count = state.loop_counts.entry(state.block_id).or_insert(0);
            *visit_count += 1;
            if *visit_count > self.config.unroll_bound {
                state.bounded = true;
                completed_paths.push(PathOutcome {
                    path_condition: state.path_conditions,
                    return_val: None,
                    memory: state.memory,
                    observable_events: state.observable_events,
                    bounded: true,
                    steps: state.step_count,
                });
                continue;
            }

            // Execute instructions in block
            for inst in &block.instructions {
                state.step_count += 1;
                if state.step_count > self.config.max_steps {
                    return Err(SymbolicError::StepLimitExceeded(self.config.max_steps));
                }

                self.exec_inst(inst, &mut state)?;
            }

            // Execute terminator
            match &block.terminator {
                WirTerminator::Return(ret_expr_opt) => {
                    let ret_val = if let Some(expr) = ret_expr_opt {
                        Some(self.eval_expr(expr, &state.env, &state.memory)?)
                    } else {
                        None
                    };
                    completed_paths.push(PathOutcome {
                        path_condition: state.path_conditions,
                        return_val: ret_val,
                        memory: state.memory,
                        observable_events: state.observable_events,
                        bounded: state.bounded,
                        steps: state.step_count,
                    });
                }
                WirTerminator::Jump(target) => {
                    state.block_id = *target;
                    worklist.push(state);
                }
                WirTerminator::Branch {
                    cond,
                    then_block,
                    else_block,
                } => {
                    let cond_val = self.eval_expr(cond, &state.env, &state.memory)?;
                    let cond_edge = cond_val.to_bool(&mut self.aig)?;

                    if cond_edge == Edge::TRUE {
                        state.observable_events.push(ObservableSymEvent::Branch {
                            cond: cond_edge,
                            taken: true,
                        });
                        state.block_id = *then_block;
                        worklist.push(state);
                    } else if cond_edge == Edge::FALSE {
                        state.observable_events.push(ObservableSymEvent::Branch {
                            cond: cond_edge,
                            taken: false,
                        });
                        state.block_id = *else_block;
                        worklist.push(state);
                    } else {
                        // Symbolic branch: fork both feasible paths
                        let mut then_state = state.clone();
                        then_state.path_conditions.push(cond_edge);
                        then_state.observable_events.push(ObservableSymEvent::Branch {
                            cond: cond_edge,
                            taken: true,
                        });
                        then_state.block_id = *then_block;

                        let mut else_state = state;
                        else_state.path_conditions.push(!cond_edge);
                        else_state.observable_events.push(ObservableSymEvent::Branch {
                            cond: cond_edge,
                            taken: false,
                        });
                        else_state.block_id = *else_block;

                        worklist.push(else_state);
                        worklist.push(then_state);
                    }
                }
                WirTerminator::Trap(msg) => {
                    return Err(SymbolicError::Trap(msg.clone()));
                }
                WirTerminator::Unreachable => {}
            }
        }

        Ok(completed_paths)
    }

    /// Execute a single WIR instruction in the current path state.
    fn exec_inst(&mut self, inst: &WirInst, state: &mut PathState) -> Result<(), SymbolicError> {
        match inst {
            WirInst::Let { var, expr, .. } => {
                let val = self.eval_expr(expr, &state.env, &state.memory)?;
                state.env.insert(var.clone(), val);
            }
            WirInst::Alloc { var, size_expr } => {
                let size_val = self.eval_expr(size_expr, &state.env, &state.memory)?;
                let size_bits = size_val.to_bits()?;
                let raw_size = bitvec_concrete_u128(&size_bits).ok_or_else(|| {
                    SymbolicError::UnsupportedOperation("Symbolic allocation size not supported".to_string())
                })?;
                let size_bytes = usize::try_from(raw_size).map_err(|_| SymbolicError::MemoryLimitExceeded {
                    requested: usize::MAX,
                    max: self.config.max_memory_bytes,
                })?;
                if size_bytes > self.config.max_memory_bytes {
                    return Err(SymbolicError::MemoryLimitExceeded {
                        requested: size_bytes,
                        max: self.config.max_memory_bytes,
                    });
                }
                let (prov, offset) = state.memory.allocate(size_bytes, None);
                state.env.insert(var.clone(), SymVal::Ptr(prov, offset));
            }
            WirInst::Load { var, ptr_expr, ty } => {
                let ptr_val = self.eval_expr(ptr_expr, &state.env, &state.memory)?;
                let (prov, offset) = match ptr_val {
                    SymVal::Ptr(p, off) => (p, off),
                    SymVal::Bits(b) => {
                        let enc = bitvec_concrete_u128(&b).ok_or_else(|| {
                            SymbolicError::TypeMismatch("Symbolic integer used as pointer".to_string())
                        })?;
                        (ProvenanceId((enc >> 32) as u64), BitVec::constant(64, enc & 0xffff_ffff))
                    }
                    _ => return Err(SymbolicError::TypeMismatch("Expected pointer for Load".to_string())),
                };

                let byte_size = ty.byte_size().unwrap_or(1);
                state.observable_events.push(ObservableSymEvent::Load {
                    provenance: prov,
                    offset: offset.clone(),
                    size: byte_size,
                });

                let loaded_val = state.memory.load(prov, &offset, ty, &mut self.aig)?;
                state.env.insert(var.clone(), loaded_val);
            }
            WirInst::Store { ptr_expr, val_expr } => {
                let ptr_val = self.eval_expr(ptr_expr, &state.env, &state.memory)?;
                let val = self.eval_expr(val_expr, &state.env, &state.memory)?;
                let (prov, offset) = match ptr_val {
                    SymVal::Ptr(p, off) => (p, off),
                    SymVal::Bits(b) => {
                        let enc = bitvec_concrete_u128(&b).ok_or_else(|| {
                            SymbolicError::TypeMismatch("Symbolic integer used as pointer".to_string())
                        })?;
                        (ProvenanceId((enc >> 32) as u64), BitVec::constant(64, enc & 0xffff_ffff))
                    }
                    _ => return Err(SymbolicError::TypeMismatch("Expected pointer for Store".to_string())),
                };

                let val_bits = val.to_bits()?;
                let raw_byte_count = val_bits.width().div_ceil(8);
                let alloc_size = state.memory.allocations.get(&prov).map(|a| a.size).unwrap_or(raw_byte_count);
                let byte_size = if raw_byte_count > alloc_size && alloc_size > 0 {
                    alloc_size
                } else {
                    raw_byte_count
                };
                state.observable_events.push(ObservableSymEvent::Store {
                    provenance: prov,
                    offset: offset.clone(),
                    size: byte_size,
                });

                state.memory.store(prov, &offset, &val, &mut self.aig)?;
            }
            WirInst::Free { .. } => {}
            WirInst::CallCapability { .. } => {
                return Err(SymbolicError::UnsupportedOperation("CallCapability in symbolic mode".to_string()));
            }
            WirInst::CallFunction { .. } => {
                return Err(SymbolicError::UnsupportedOperation("Uninlined function call in symbolic mode".to_string()));
            }
        }
        Ok(())
    }

    /// Evaluate an expression symbolically.
    pub fn eval_expr(
        &mut self,
        expr: &WirExpr,
        env: &HashMap<VarId, SymVal>,
        memory: &SymMemory,
    ) -> Result<SymVal, SymbolicError> {
        match expr {
            WirExpr::ConstInt { val, ty } => {
                let w = ty.bit_width().unwrap_or(64) as usize;
                Ok(SymVal::Bits(BitVec::constant(w, *val as u128)))
            }
            WirExpr::ConstBool(b) => Ok(SymVal::Bool(Edge::constant(*b))),
            WirExpr::ConstBytes(bytes) => {
                // Return pointer to a newly created static allocation
                let mut bits_vec = Vec::new();
                for &b in bytes {
                    bits_vec.push(BitVec::constant(8, b as u128));
                }
                // When ConstBytes is in an expression, lower as a tuple of bytes or constant bits
                if bytes.len() <= 8 {
                    let mut combined = BitVec::constant(bytes.len() * 8, 0);
                    for (i, &b) in bytes.iter().enumerate() {
                        let b_bv = BitVec::constant(8, b as u128);
                        let shifted = bv::shl_const(&zero_extend(&b_bv, bytes.len() * 8), i * 8);
                        combined = bv::or(&mut self.aig, &combined, &shifted);
                    }
                    Ok(SymVal::Bits(combined))
                } else {
                    Err(SymbolicError::UnsupportedOperation("ConstBytes > 8 bytes".to_string()))
                }
            }
            WirExpr::ConstUnit => Ok(SymVal::Unit),
            WirExpr::Var(v) => env
                .get(v)
                .cloned()
                .ok_or_else(|| SymbolicError::UndefinedVariable(v.clone())),
            WirExpr::Unary { op, expr: inner } => {
                let inner_val = self.eval_expr(inner, env, memory)?;
                match op {
                    UnaryOp::Not => {
                        if let SymVal::Bool(e) = inner_val {
                            Ok(SymVal::Bool(!e))
                        } else {
                            let b = inner_val.to_bits()?;
                            Ok(SymVal::Bits(bv::not(&b)))
                        }
                    }
                    UnaryOp::Neg(_) => {
                        let b = inner_val.to_bits()?;
                        Ok(SymVal::Bits(bv::neg(&mut self.aig, &b)))
                    }
                }
            }
            WirExpr::Binary { op, lhs, rhs } => {
                let l_val = self.eval_expr(lhs, env, memory)?;
                let r_val = self.eval_expr(rhs, env, memory)?;

                // Boolean logic shortcut
                if let (SymVal::Bool(l_edge), SymVal::Bool(r_edge)) = (&l_val, &r_val) {
                    match op {
                        BinaryOp::BitAnd => return Ok(SymVal::Bool(self.aig.and(*l_edge, *r_edge))),
                        BinaryOp::BitOr => return Ok(SymVal::Bool(self.aig.or(*l_edge, *r_edge))),
                        BinaryOp::BitXor => return Ok(SymVal::Bool(self.aig.xor(*l_edge, *r_edge))),
                        BinaryOp::Eq => return Ok(SymVal::Bool(self.aig.xnor(*l_edge, *r_edge))),
                        BinaryOp::Ne => return Ok(SymVal::Bool(self.aig.xor(*l_edge, *r_edge))),
                        _ => {}
                    }
                }

                let mut l_bv = l_val.to_bits()?;
                let mut r_bv = r_val.to_bits()?;

                // Align widths to max_w
                if l_bv.width() != r_bv.width() {
                    let max_w = std::cmp::max(l_bv.width(), r_bv.width());
                    l_bv = zero_extend(&l_bv, max_w);
                    r_bv = zero_extend(&r_bv, max_w);
                }

                match op {
                    BinaryOp::Add(_) => Ok(SymVal::Bits(bv::add(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Sub(_) => Ok(SymVal::Bits(bv::sub(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Mul(_) => Ok(SymVal::Bits(bv::mul(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::BitAnd => Ok(SymVal::Bits(bv::and(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::BitOr => Ok(SymVal::Bits(bv::or(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::BitXor => Ok(SymVal::Bits(bv::xor(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Shl => Ok(SymVal::Bits(bv::shl(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Shr => Ok(SymVal::Bits(bv::lshr(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Eq => Ok(SymVal::Bool(bv::eq(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Ne => Ok(SymVal::Bool(!bv::eq(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Lt => Ok(SymVal::Bool(bv::ult(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Le => Ok(SymVal::Bool(bv::ule(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Gt => Ok(SymVal::Bool(bv::ugt(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Ge => Ok(SymVal::Bool(bv::uge(&mut self.aig, &l_bv, &r_bv))),
                    BinaryOp::Div(_) | BinaryOp::Rem(_) => Err(SymbolicError::UnsupportedOperation(
                        "Symbolic division / remainder not supported".to_string(),
                    )),
                }
            }
            WirExpr::Trunc { expr: inner, target_ty } => {
                let b = self.eval_expr(inner, env, memory)?.to_bits()?;
                let target_w = target_ty.bit_width().unwrap_or(b.width() as u32) as usize;
                if target_w > b.width() {
                    return Err(SymbolicError::TypeMismatch(format!(
                        "Cannot truncate from width {} to larger width {}",
                        b.width(),
                        target_w
                    )));
                }
                Ok(SymVal::Bits(truncate(&b, target_w)))
            }
            WirExpr::ZExt { expr: inner, target_ty } => {
                let b = self.eval_expr(inner, env, memory)?.to_bits()?;
                let target_w = target_ty.bit_width().unwrap_or(b.width() as u32) as usize;
                Ok(SymVal::Bits(zero_extend(&b, target_w)))
            }
            WirExpr::SExt { expr: inner, target_ty } => {
                let b = self.eval_expr(inner, env, memory)?.to_bits()?;
                let target_w = target_ty.bit_width().unwrap_or(b.width() as u32) as usize;
                Ok(SymVal::Bits(sign_extend(&b, target_w)))
            }
            WirExpr::TupleMake(elems) => {
                let mut vals = Vec::new();
                for e in elems {
                    vals.push(self.eval_expr(e, env, memory)?);
                }
                Ok(SymVal::Tuple(vals))
            }
            WirExpr::TupleGet { expr: inner, index } => {
                let inner_val = self.eval_expr(inner, env, memory)?;
                if let SymVal::Tuple(elems) = inner_val {
                    elems
                        .get(*index)
                        .cloned()
                        .ok_or_else(|| SymbolicError::TypeMismatch("Tuple index out of bounds".to_string()))
                } else {
                    Err(SymbolicError::TypeMismatch("Expected tuple in TupleGet".to_string()))
                }
            }
            WirExpr::PtrOffset { ptr, offset_bytes } => {
                let ptr_val = self.eval_expr(ptr, env, memory)?;
                let off_val = self.eval_expr(offset_bytes, env, memory)?;
                let off_bv = off_val.to_bits()?;
                match ptr_val {
                    SymVal::Ptr(prov, base_off) => {
                        let off_64 = if off_bv.width() == 64 {
                            off_bv
                        } else if off_bv.width() < 64 {
                            zero_extend(&off_bv, 64)
                        } else {
                            truncate(&off_bv, 64)
                        };
                        let new_off = bv::add(&mut self.aig, &base_off, &off_64);
                        Ok(SymVal::Ptr(prov, new_off))
                    }
                    _ => Err(SymbolicError::TypeMismatch("Expected pointer in PtrOffset".to_string())),
                }
            }
            WirExpr::PtrLen(ptr) => {
                let ptr_val = self.eval_expr(ptr, env, memory)?;
                match ptr_val {
                    SymVal::Ptr(prov, _) => {
                        let alloc = memory.allocations.get(&prov).ok_or_else(|| {
                            SymbolicError::TypeMismatch(format!("Invalid provenance {:?} in PtrLen", prov))
                        })?;
                        Ok(SymVal::Bits(BitVec::constant(64, alloc.size as u128)))
                    }
                    _ => Err(SymbolicError::TypeMismatch("Expected pointer in PtrLen".to_string())),
                }
            }
            WirExpr::MakeDynamic { .. } | WirExpr::CheckDynamic { .. } => {
                Err(SymbolicError::UnsupportedOperation("Dynamic typing in symbolic mode".to_string()))
            }
        }
    }
}

/// Verification verdict rendered by the bit-precise prover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationVerdict {
    /// Completely proved for all inputs.
    Proved,
    /// Bounded proof up to loop unroll limit `k`.
    Bounded(usize),
    /// Falsified with concrete counterexample inputs.
    Falsified {
        counterexample: Vec<(String, u128)>,
    },
    /// Inconclusive or limit exceeded.
    Unknown(String),
}

/// Bit-precise prover discharging WIR claims and invariants via `cdcl-sat`.
pub struct Prover {
    /// Verification executor.
    pub executor: SymbolicExecutor,
}

impl Default for Prover {
    fn default() -> Self {
        Self::new()
    }
}

impl Prover {
    /// Create a new Prover instance.
    pub fn new() -> Self {
        Self {
            executor: SymbolicExecutor::new(),
        }
    }

    /// Create a new Prover with customized config.
    pub fn with_config(config: SymbolicConfig) -> Self {
        Self {
            executor: SymbolicExecutor::with_config(config),
        }
    }

    /// Prove that a function's return value equals a specification circuit for all inputs.
    pub fn prove_equivalence<F>(
        &mut self,
        func: &Function,
        args: Vec<SymVal>,
        initial_memory: SymMemory,
        spec_builder: F,
    ) -> VerificationVerdict
    where
        F: FnOnce(&mut Aig, &[SymVal], &SymMemory) -> Result<SymVal, SymbolicError>,
    {
        let spec_res = match spec_builder(&mut self.executor.aig, &args, &initial_memory) {
            Ok(v) => v,
            Err(e) => return VerificationVerdict::Unknown(format!("Spec builder error: {}", e)),
        };

        let outcomes = match self.executor.execute_function(func, args, initial_memory) {
            Ok(o) => o,
            Err(e) => return VerificationVerdict::Unknown(format!("Symbolic execution error: {}", e)),
        };

        let mut any_bounded = false;
        let mut miter_clauses = Vec::new();

        for outcome in outcomes {
            if outcome.bounded {
                any_bounded = true;
            }

            let ret_val = match outcome.return_val {
                Some(v) => v,
                None => continue,
            };

            // Assert: path_condition => (ret_val == spec_res)
            // Miter goal: path_condition ∧ (ret_val != spec_res)
            let eq_edge = match (&ret_val, &spec_res) {
                (SymVal::Bool(r_edge), SymVal::Bool(s_edge)) => {
                    self.executor.aig.xnor(*r_edge, *s_edge)
                }
                _ => {
                    let r_bv = match ret_val.to_bits() {
                        Ok(b) => b,
                        Err(e) => return VerificationVerdict::Unknown(format!("Ret val bits error: {}", e)),
                    };
                    let s_bv = match spec_res.to_bits() {
                        Ok(b) => b,
                        Err(e) => return VerificationVerdict::Unknown(format!("Spec val bits error: {}", e)),
                    };
                    if r_bv.width() != s_bv.width() {
                        return VerificationVerdict::Unknown(format!(
                            "Return bit-width mismatch between impl ({}) and spec ({})",
                            r_bv.width(),
                            s_bv.width()
                        ));
                    }
                    bv::eq(&mut self.executor.aig, &r_bv, &s_bv)
                }
            };

            let ne_edge = !eq_edge;
            let mut path_and_ne = outcome.path_condition.clone();
            path_and_ne.push(ne_edge);
            let path_miter = self.executor.aig.and_all(&path_and_ne);
            miter_clauses.push(path_miter);
        }

        if miter_clauses.is_empty() {
            return VerificationVerdict::Unknown("No executable paths returned".to_string());
        }

        // Overall miter: is any path's (path_condition ∧ ne) satisfiable?
        let overall_miter = self.executor.aig.or_all(&miter_clauses);

        let mut cnf = Cnf::new(0);
        let mut tseitin = Tseitin::new();
        tseitin.assert_true(&self.executor.aig, overall_miter, &mut cnf);

        let mut solver = Solver::from_cnf_with_proof(&cnf);
        if let Some(budget) = self.executor.config.solver_conflict_budget {
            solver.set_conflict_budget(Some(budget));
        }
        match solver.solve() {
            SolveResult::Unsat => {
                // Verify DRAT certificate
                if let Some(proof) = solver.proof() {
                    let proof_text = proof.to_drat_text();
                    let dimacs = cdcl_sat::write_dimacs(&cnf);
                    if let Err(e) = drat_check::check(&dimacs, &proof_text) {
                        return VerificationVerdict::Unknown(format!("DRAT proof rejected: {}", e));
                    }
                }

                if any_bounded {
                    VerificationVerdict::Bounded(self.executor.config.unroll_bound)
                } else {
                    VerificationVerdict::Proved
                }
            }
            SolveResult::Sat => {
                let model = solver.model();
                let mut counterexample = Vec::new();
                for (name, bv) in &self.executor.input_vars {
                    let mut val = 0u128;
                    for (i, edge) in bv.bits.iter().enumerate() {
                        if let Node::Input(input_idx) = self.executor.aig.node(edge.node()) {
                            if let Some(b) = tseitin.input_value(&self.executor.aig, input_idx, model) {
                                if b ^ edge.is_complemented() && i < 128 {
                                    val |= 1 << i;
                                }
                            }
                        }
                    }
                    counterexample.push((name.clone(), val));
                }
                VerificationVerdict::Falsified { counterexample }
            }
            SolveResult::Unknown => {
                VerificationVerdict::Unknown("SAT solver search limit exceeded".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BasicBlock, Label, OverflowMode};

    #[test]
    fn test_bitvec_extensions_and_concatenation() {
        let bv = BitVec::constant(8, 0x85); // 1000 0101
        assert_eq!(bitvec_concrete_u128(&bv), Some(0x85));

        let zext = zero_extend(&bv, 16);
        assert_eq!(zext.width(), 16);
        assert_eq!(bitvec_concrete_u128(&zext), Some(0x0085));

        let sext = sign_extend(&bv, 16);
        assert_eq!(sext.width(), 16);
        assert_eq!(bitvec_concrete_u128(&sext), Some(0xff85));

        let trunc = truncate(&bv, 4);
        assert_eq!(trunc.width(), 4);
        assert_eq!(bitvec_concrete_u128(&trunc), Some(0x05));

        let high = BitVec::constant(8, 0x12);
        let cat = concat(&bv, &high);
        assert_eq!(cat.width(), 16);
        assert_eq!(bitvec_concrete_u128(&cat), Some(0x1285));
    }

    #[test]
    fn test_symval_conversions() {
        let mut aig = Aig::new();
        let val_bits = SymVal::constant_bits(32, 42);
        assert_eq!(val_bits.width(), Some(32));
        assert!(val_bits.to_bits().is_ok());
        let bool_edge = val_bits.to_bool(&mut aig).unwrap();
        assert_eq!(bool_edge, Edge::TRUE);

        let val_zero = SymVal::constant_bits(32, 0);
        assert_eq!(val_zero.to_bool(&mut aig).unwrap(), Edge::FALSE);

        let val_bool = SymVal::constant_bool(true);
        assert_eq!(val_bool.width(), Some(1));
        assert_eq!(val_bool.to_bool(&mut aig).unwrap(), Edge::TRUE);

        let err_bits = SymVal::Unit.to_bits();
        assert!(err_bits.is_err());
        let err_bool = SymVal::Unit.to_bool(&mut aig);
        assert!(err_bool.is_err());
    }

    #[test]
    fn test_sym_memory_concrete_and_symbolic() {
        let mut aig = Aig::new();
        let mut mem = SymMemory::new();

        // Concrete allocation
        let (prov1, off1) = mem.allocate(16, Some(0xAA));
        assert_eq!(mem.allocations[&prov1].size, 16);

        // Load 1 byte
        let loaded = mem.load(prov1, &off1, &WirType::U8, &mut aig).unwrap();
        assert_eq!(bitvec_concrete_u128(&loaded.to_bits().unwrap()), Some(0xAA));

        // Store 2 bytes
        let val_to_store = SymVal::constant_bits(16, 0x1234);
        let store_off = BitVec::constant(64, 4);
        mem.store(prov1, &store_off, &val_to_store, &mut aig).unwrap();

        // Load 2 bytes from offset 4
        let loaded2 = mem.load(prov1, &store_off, &WirType::U16, &mut aig).unwrap();
        assert_eq!(bitvec_concrete_u128(&loaded2.to_bits().unwrap()), Some(0x1234));

        // Out of bounds check
        let bad_off = BitVec::constant(64, 15);
        let bad_load = mem.load(prov1, &bad_off, &WirType::U16, &mut aig);
        assert!(matches!(bad_load, Err(SymbolicError::OutOfBounds { .. })));

        // Symbolic allocation and symbolic store
        let (prov2, _) = mem.allocate_symbolic(4, &mut aig);
        let sym_off = BitVec::input(&mut aig, 64);
        let sym_val = SymVal::Bits(BitVec::input(&mut aig, 8));
        let store_sym = mem.store(prov2, &sym_off, &sym_val, &mut aig);
        assert!(store_sym.is_ok());

        let load_sym = mem.load(prov2, &sym_off, &WirType::U8, &mut aig);
        assert!(load_sym.is_ok());
    }

    #[test]
    fn test_symbolic_executor_straight_line() {
        let mut exec = SymbolicExecutor::new();
        let a = exec.create_input_bits("a", 64);
        let b = exec.create_input_bits("b", 64);

        let func = Function {
            name: "add_bitwise".to_string(),
            params: vec![
                (VarId::from("a"), WirType::U64, Label::PUBLIC),
                (VarId::from("b"), WirType::U64, Label::PUBLIC),
            ],
            return_type: WirType::U64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    WirInst::Let {
                        var: VarId::from("sum"),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::Add(OverflowMode::Wrap),
                            lhs: Box::new(WirExpr::Var(VarId::from("a"))),
                            rhs: Box::new(WirExpr::Var(VarId::from("b"))),
                        },
                    },
                    WirInst::Let {
                        var: VarId::from("res"),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::BitXor,
                            lhs: Box::new(WirExpr::Var(VarId::from("sum"))),
                            rhs: Box::new(WirExpr::ConstInt {
                                val: 0xFF,
                                ty: WirType::U64,
                            }),
                        },
                    },
                ],
                terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("res")))),
            }],
            ranking_metric: None,
        };

        let outcomes = exec
            .execute_function(
                &func,
                vec![SymVal::Bits(a.clone()), SymVal::Bits(b.clone())],
                SymMemory::new(),
            )
            .unwrap();

        assert_eq!(outcomes.len(), 1);
        assert!(!outcomes[0].bounded);
        assert!(outcomes[0].return_val.is_some());
    }

    #[test]
    fn test_prover_proves_xor_identity() {
        let mut prover = Prover::new();
        let a = prover.executor.create_input_bits("a", 64);
        let b = prover.executor.create_input_bits("b", 64);

        // a ^ b ^ b == a
        let func = Function {
            name: "xor_cancel".to_string(),
            params: vec![
                (VarId::from("a"), WirType::U64, Label::PUBLIC),
                (VarId::from("b"), WirType::U64, Label::PUBLIC),
            ],
            return_type: WirType::U64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    WirInst::Let {
                        var: VarId::from("tmp"),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::BitXor,
                            lhs: Box::new(WirExpr::Var(VarId::from("a"))),
                            rhs: Box::new(WirExpr::Var(VarId::from("b"))),
                        },
                    },
                    WirInst::Let {
                        var: VarId::from("res"),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::Binary {
                            op: BinaryOp::BitXor,
                            lhs: Box::new(WirExpr::Var(VarId::from("tmp"))),
                            rhs: Box::new(WirExpr::Var(VarId::from("b"))),
                        },
                    },
                ],
                terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("res")))),
            }],
            ranking_metric: None,
        };

        let verdict = prover.prove_equivalence(
            &func,
            vec![SymVal::Bits(a.clone()), SymVal::Bits(b)],
            SymMemory::new(),
            |_aig, args, _mem| Ok(args[0].clone()), // spec is just `a`
        );

        assert_eq!(verdict, VerificationVerdict::Proved);
    }

    #[test]
    fn test_prover_falsifies_flawed_implementation() {
        let mut prover = Prover::new();
        let a = prover.executor.create_input_bits("a", 16);
        let b = prover.executor.create_input_bits("b", 16);

        // Flawed implementation: `a + b` instead of `a ^ b`
        let func = Function {
            name: "flawed_xor".to_string(),
            params: vec![
                (VarId::from("a"), WirType::U16, Label::PUBLIC),
                (VarId::from("b"), WirType::U16, Label::PUBLIC),
            ],
            return_type: WirType::U16,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::Let {
                    var: VarId::from("res"),
                    ty: WirType::U16,
                    label: Label::PUBLIC,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Add(OverflowMode::Wrap),
                        lhs: Box::new(WirExpr::Var(VarId::from("a"))),
                        rhs: Box::new(WirExpr::Var(VarId::from("b"))),
                    },
                }],
                terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("res")))),
            }],
            ranking_metric: None,
        };

        let verdict = prover.prove_equivalence(
            &func,
            vec![SymVal::Bits(a.clone()), SymVal::Bits(b.clone())],
            SymMemory::new(),
            |aig, args, _mem| {
                let a_b = args[0].to_bits().unwrap();
                let b_b = args[1].to_bits().unwrap();
                Ok(SymVal::Bits(bv::xor(aig, &a_b, &b_b)))
            },
        );

        assert!(matches!(verdict, VerificationVerdict::Falsified { .. }));
    }
}
