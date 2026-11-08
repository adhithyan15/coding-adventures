//! Deterministic reference interpreter for the Warrant Intermediate Representation (WIR).

use std::collections::HashMap;

use crate::ast::*;
use crate::cost::{CostSchedule, ExecutionCost};
use crate::trace::{AccessKind, ObservableTrace};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WirValue {
    Unit,
    Bool(bool),
    Int { val: i128, ty: WirType },
    Bytes(Vec<u8>),
    Ptr { provenance: ProvenanceId, offset: u64 },
    Tuple(Vec<WirValue>),
    Dynamic { tag: String, inner: Box<WirValue> },
}

impl WirValue {
    pub fn bool(b: bool) -> Self {
        WirValue::Bool(b)
    }

    pub fn u8(val: u8) -> Self {
        WirValue::Int {
            val: val as i128,
            ty: WirType::U8,
        }
    }

    pub fn u64(val: u64) -> Self {
        WirValue::Int {
            val: val as i128,
            ty: WirType::U64,
        }
    }

    pub fn ptr(provenance: ProvenanceId, offset: u64) -> Self {
        WirValue::Ptr { provenance, offset }
    }

    pub fn as_bool(&self) -> Result<bool, UndefinedBehavior> {
        match self {
            WirValue::Bool(b) => Ok(*b),
            _ => Err(UndefinedBehavior::TypeMismatch("Expected bool".to_string())),
        }
    }

    pub fn as_int(&self) -> Result<(i128, &WirType), UndefinedBehavior> {
        match self {
            WirValue::Int { val, ty } => Ok((*val, ty)),
            _ => Err(UndefinedBehavior::TypeMismatch("Expected integer".to_string())),
        }
    }

    pub fn as_ptr(&self) -> Result<(ProvenanceId, u64), UndefinedBehavior> {
        match self {
            WirValue::Ptr { provenance, offset } => Ok((*provenance, *offset)),
            _ => Err(UndefinedBehavior::TypeMismatch("Expected pointer".to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndefinedBehavior {
    OutOfBounds {
        provenance: ProvenanceId,
        offset: u64,
        size: usize,
        bound: usize,
    },
    InvalidProvenance(ProvenanceId),
    UseAfterFree(ProvenanceId),
    DivisionByZero,
    ShiftOverflow,
    TypeMismatch(String),
    DynamicTagMismatch { expected: String, actual: String },
    UnreachableHit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetLimit {
    StepLimitExceeded(u64),
    CallDepthExceeded(usize),
    MemoryLimitExceeded(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionOutcome {
    Success {
        value: WirValue,
        cost: ExecutionCost,
        trace: ObservableTrace,
    },
    Trap {
        reason: String,
        cost: ExecutionCost,
    },
    Stuck {
        reason: UndefinedBehavior,
        cost: ExecutionCost,
    },
    BudgetExceeded(BudgetLimit),
}

impl ExecutionOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionOutcome::Success { .. })
    }

    pub fn value(&self) -> Option<&WirValue> {
        match self {
            ExecutionOutcome::Success { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn unwrap_value(self) -> WirValue {
        match self {
            ExecutionOutcome::Success { value, .. } => value,
            ExecutionOutcome::Trap { reason, .. } => panic!("Execution trapped: {reason}"),
            ExecutionOutcome::Stuck { reason, .. } => panic!("Execution stuck: {reason:?}"),
            ExecutionOutcome::BudgetExceeded(b) => panic!("Budget exceeded: {b:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub id: ProvenanceId,
    pub size: usize,
    pub bytes: Vec<u8>,
    pub alive: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Memory {
    pub allocations: HashMap<ProvenanceId, Allocation>,
    next_id: u64,
    total_allocated_bytes: usize,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            next_id: 1,
            total_allocated_bytes: 0,
        }
    }

    pub fn allocate(
        &mut self,
        size: usize,
        max_bytes: usize,
    ) -> Result<ProvenanceId, BudgetLimit> {
        if self.total_allocated_bytes.saturating_add(size) > max_bytes {
            return Err(BudgetLimit::MemoryLimitExceeded(max_bytes));
        }
        let id = ProvenanceId(self.next_id);
        self.next_id += 1;
        self.total_allocated_bytes += size;
        self.allocations.insert(
            id,
            Allocation {
                id,
                size,
                bytes: vec![0u8; size],
                alive: true,
            },
        );
        Ok(id)
    }

    pub fn free(&mut self, id: ProvenanceId) -> Result<(), UndefinedBehavior> {
        let alloc = self
            .allocations
            .get_mut(&id)
            .ok_or(UndefinedBehavior::InvalidProvenance(id))?;
        if !alloc.alive {
            return Err(UndefinedBehavior::UseAfterFree(id));
        }
        alloc.alive = false;
        Ok(())
    }

    pub fn load_bytes(
        &self,
        id: ProvenanceId,
        offset: u64,
        size: usize,
    ) -> Result<Vec<u8>, UndefinedBehavior> {
        let alloc = self
            .allocations
            .get(&id)
            .ok_or(UndefinedBehavior::InvalidProvenance(id))?;
        if !alloc.alive {
            return Err(UndefinedBehavior::UseAfterFree(id));
        }
        let start = offset as usize;
        let end = start.saturating_add(size);
        if end > alloc.size {
            return Err(UndefinedBehavior::OutOfBounds {
                provenance: id,
                offset,
                size,
                bound: alloc.size,
            });
        }
        Ok(alloc.bytes[start..end].to_vec())
    }

    pub fn store_bytes(
        &mut self,
        id: ProvenanceId,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), UndefinedBehavior> {
        let size = bytes.len();
        let alloc = self
            .allocations
            .get_mut(&id)
            .ok_or(UndefinedBehavior::InvalidProvenance(id))?;
        if !alloc.alive {
            return Err(UndefinedBehavior::UseAfterFree(id));
        }
        let start = offset as usize;
        let end = start.saturating_add(size);
        if end > alloc.size {
            return Err(UndefinedBehavior::OutOfBounds {
                provenance: id,
                offset,
                size,
                bound: alloc.size,
            });
        }
        alloc.bytes[start..end].copy_from_slice(bytes);
        Ok(())
    }
}

pub trait CapabilityDispatcher {
    fn call(
        &mut self,
        name: &str,
        args: &[WirValue],
    ) -> Result<WirValue, String>;
}

#[derive(Default, Clone, Copy, Debug)]
pub struct DefaultCapabilityDispatcher;

impl CapabilityDispatcher for DefaultCapabilityDispatcher {
    fn call(&mut self, name: &str, args: &[WirValue]) -> Result<WirValue, String> {
        match name {
            "rand" => {
                let n = if let Some(first) = args.first() {
                    first.as_int().map(|(v, _)| v as u64).unwrap_or(0)
                } else {
                    0
                };
                Ok(WirValue::Int {
                    val: (n ^ 0x5a5a5a5a) as i128,
                    ty: WirType::U64,
                })
            }
            "host_log" => Ok(WirValue::Unit),
            other => Err(format!("Unknown capability: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterpreterConfig {
    pub max_steps: u64,
    pub max_call_depth: usize,
    pub max_memory_bytes: usize,
    pub schedule: CostSchedule,
}

impl Default for InterpreterConfig {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_call_depth: 64,
            max_memory_bytes: 16 * 1024 * 1024,
            schedule: CostSchedule::default(),
        }
    }
}

pub struct Interpreter<'a, D: CapabilityDispatcher = DefaultCapabilityDispatcher> {
    program: &'a Program,
    config: InterpreterConfig,
    memory: Memory,
    cost: ExecutionCost,
    trace: ObservableTrace,
    dispatcher: D,
}

impl<'a> Interpreter<'a, DefaultCapabilityDispatcher> {
    pub fn new(program: &'a Program) -> Self {
        Self::with_config(program, InterpreterConfig::default(), DefaultCapabilityDispatcher)
    }
}

impl<'a, D: CapabilityDispatcher> Interpreter<'a, D> {
    pub fn with_config(
        program: &'a Program,
        config: InterpreterConfig,
        dispatcher: D,
    ) -> Self {
        Self {
            program,
            config,
            memory: Memory::new(),
            cost: ExecutionCost::new(),
            trace: ObservableTrace::new(),
            dispatcher,
        }
    }

    pub fn run(
        &mut self,
        entry_function: &str,
        args: &[WirValue],
    ) -> ExecutionOutcome {
        self.execute(entry_function, args.to_vec())
    }

    pub fn memory(&self) -> &Memory {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    pub fn cost(&self) -> &ExecutionCost {
        &self.cost
    }

    pub fn trace(&self) -> &ObservableTrace {
        &self.trace
    }

    pub fn allocate_bytes(&mut self, bytes: &[u8]) -> Result<WirValue, BudgetLimit> {
        let prov = self.memory.allocate(bytes.len(), self.config.max_memory_bytes)?;
        let _ = self.memory.store_bytes(prov, 0, bytes);
        Ok(WirValue::Ptr {
            provenance: prov,
            offset: 0,
        })
    }

    pub fn read_bytes(&self, ptr: &WirValue, len: usize) -> Result<Vec<u8>, UndefinedBehavior> {
        let (prov, offset) = ptr.as_ptr()?;
        self.memory.load_bytes(prov, offset, len)
    }

    pub fn execute(
        &mut self,
        entry_function: &str,
        args: Vec<WirValue>,
    ) -> ExecutionOutcome {
        let func = match self.program.find_function(entry_function) {
            Some(f) => f,
            None => {
                return ExecutionOutcome::Trap {
                    reason: format!("Entry function '{entry_function}' not found"),
                    cost: self.cost,
                };
            }
        };

        if func.params.len() != args.len() {
            return ExecutionOutcome::Trap {
                reason: format!(
                    "Argument count mismatch: expected {}, got {}",
                    func.params.len(),
                    args.len()
                ),
                cost: self.cost,
            };
        }

        self.call_fn(func, args, 1)
    }

    fn call_fn(
        &mut self,
        func: &Function,
        args: Vec<WirValue>,
        depth: usize,
    ) -> ExecutionOutcome {
        if depth > self.config.max_call_depth {
            return ExecutionOutcome::BudgetExceeded(BudgetLimit::CallDepthExceeded(depth));
        }

        self.cost.add_steps(self.config.schedule.call_cost);
        if self.cost.steps > self.config.max_steps {
            return ExecutionOutcome::BudgetExceeded(BudgetLimit::StepLimitExceeded(self.cost.steps));
        }

        let mut env: HashMap<VarId, WirValue> = HashMap::new();
        for ((pvar, _pty, _plabel), arg_val) in func.params.iter().zip(args) {
            env.insert(pvar.clone(), arg_val);
        }

        let mut current_block_id = BlockId(0);

        loop {
            let block = match func.blocks.iter().find(|b| b.id == current_block_id) {
                Some(b) => b,
                None => {
                    return ExecutionOutcome::Stuck {
                        reason: UndefinedBehavior::TypeMismatch(format!(
                            "Block {current_block_id:?} not found in function {}",
                            func.name
                        )),
                        cost: self.cost,
                    };
                }
            };

            for inst in &block.instructions {
                self.cost.add_steps(self.config.schedule.alu_step_cost);
                if self.cost.steps > self.config.max_steps {
                    return ExecutionOutcome::BudgetExceeded(BudgetLimit::StepLimitExceeded(
                        self.cost.steps,
                    ));
                }

                match inst {
                    WirInst::Let { var, expr, .. } => {
                        let val = match self.eval_expr(expr, &env) {
                            Ok(v) => v,
                            Err(ub) => {
                                return ExecutionOutcome::Stuck {
                                    reason: ub,
                                    cost: self.cost,
                                };
                            }
                        };
                        env.insert(var.clone(), val);
                    }
                    WirInst::Alloc { var, size_expr } => {
                        self.cost.add_steps(self.config.schedule.alloc_cost);
                        let size_val = match self.eval_expr(size_expr, &env) {
                            Ok(v) => v,
                            Err(ub) => {
                                return ExecutionOutcome::Stuck {
                                    reason: ub,
                                    cost: self.cost,
                                };
                            }
                        };
                        let (size_int, _) = match size_val.as_int() {
                            Ok(i) => i,
                            Err(ub) => {
                                return ExecutionOutcome::Stuck {
                                    reason: ub,
                                    cost: self.cost,
                                };
                            }
                        };
                        if size_int < 0 {
                            return ExecutionOutcome::Stuck {
                                reason: UndefinedBehavior::TypeMismatch("Negative allocation size".to_string()),
                                cost: self.cost,
                            };
                        }
                        let size_usize = size_int as usize;
                        let prov = match self.memory.allocate(size_usize, self.config.max_memory_bytes) {
                            Ok(p) => p,
                            Err(limit) => return ExecutionOutcome::BudgetExceeded(limit),
                        };
                        self.cost.allocations += 1;
                        self.cost.allocated_bytes += size_usize as u64;
                        env.insert(
                            var.clone(),
                            WirValue::Ptr {
                                provenance: prov,
                                offset: 0,
                            },
                        );
                    }
                    WirInst::Load { var, ptr_expr, ty } => {
                        self.cost.add_steps(self.config.schedule.memory_read_cost);
                        self.cost.memory_reads += 1;
                        let ptr_val = match self.eval_expr(ptr_expr, &env) {
                            Ok(v) => v,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        let (prov, offset) = match ptr_val.as_ptr() {
                            Ok(p) => p,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        let size = ty.byte_size().unwrap_or(1);
                        self.trace.record_memory(prov, offset, AccessKind::Read, size);

                        let raw_bytes = match self.memory.load_bytes(prov, offset, size) {
                            Ok(b) => b,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        let val = match decode_value(&raw_bytes, ty) {
                            Ok(v) => v,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        env.insert(var.clone(), val);
                    }
                    WirInst::Store { ptr_expr, val_expr } => {
                        self.cost.add_steps(self.config.schedule.memory_write_cost);
                        self.cost.memory_writes += 1;
                        let ptr_val = match self.eval_expr(ptr_expr, &env) {
                            Ok(v) => v,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        let val = match self.eval_expr(val_expr, &env) {
                            Ok(v) => v,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        let (prov, offset) = match ptr_val.as_ptr() {
                            Ok(p) => p,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        let encoded = match encode_value(&val) {
                            Ok(b) => b,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        self.trace.record_memory(prov, offset, AccessKind::Write, encoded.len());
                        if let Err(ub) = self.memory.store_bytes(prov, offset, &encoded) {
                            return ExecutionOutcome::Stuck { reason: ub, cost: self.cost };
                        }
                    }
                    WirInst::Free { ptr_expr } => {
                        self.cost.add_steps(self.config.schedule.free_cost);
                        let ptr_val = match self.eval_expr(ptr_expr, &env) {
                            Ok(v) => v,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        let (prov, _) = match ptr_val.as_ptr() {
                            Ok(p) => p,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        };
                        if let Err(ub) = self.memory.free(prov) {
                            return ExecutionOutcome::Stuck { reason: ub, cost: self.cost };
                        }
                    }
                    WirInst::CallCapability { var, capability, args } => {
                        self.cost.add_steps(self.config.schedule.capability_cost);
                        self.cost.capability_calls += 1;
                        self.trace.record_capability(capability);
                        let mut evaluated_args = Vec::new();
                        for a in args {
                            match self.eval_expr(a, &env) {
                                Ok(v) => evaluated_args.push(v),
                                Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                            }
                        }
                        match self.dispatcher.call(capability, &evaluated_args) {
                            Ok(ret) => {
                                if let Some(v) = var {
                                    env.insert(v.clone(), ret);
                                }
                            }
                            Err(err_msg) => {
                                return ExecutionOutcome::Trap {
                                    reason: err_msg,
                                    cost: self.cost,
                                };
                            }
                        }
                    }
                    WirInst::CallFunction { var, func_name, args } => {
                        let target_func = match self.program.find_function(func_name) {
                            Some(f) => f,
                            None => {
                                return ExecutionOutcome::Trap {
                                    reason: format!("Function '{func_name}' not found"),
                                    cost: self.cost,
                                };
                            }
                        };
                        let mut evaluated_args = Vec::new();
                        for a in args {
                            match self.eval_expr(a, &env) {
                                Ok(v) => evaluated_args.push(v),
                                Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                            }
                        }
                        let outcome = self.call_fn(target_func, evaluated_args, depth + 1);
                        match outcome {
                            ExecutionOutcome::Success { value, .. } => {
                                if let Some(v) = var {
                                    env.insert(v.clone(), value);
                                }
                            }
                            other => return other,
                        }
                    }
                }
            }

            match &block.terminator {
                WirTerminator::Return(maybe_expr) => {
                    self.cost.add_steps(self.config.schedule.return_cost);
                    let ret_val = if let Some(e) = maybe_expr {
                        match self.eval_expr(e, &env) {
                            Ok(v) => v,
                            Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                        }
                    } else {
                        WirValue::Unit
                    };
                    return ExecutionOutcome::Success {
                        value: ret_val,
                        cost: self.cost,
                        trace: self.trace.clone(),
                    };
                }
                WirTerminator::Jump(target) => {
                    self.cost.add_steps(self.config.schedule.jump_step_cost);
                    current_block_id = *target;
                }
                WirTerminator::Branch { cond, then_block, else_block } => {
                    self.cost.add_steps(self.config.schedule.branch_step_cost);
                    let cond_val = match self.eval_expr(cond, &env) {
                        Ok(v) => v,
                        Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                    };
                    let b = match cond_val.as_bool() {
                        Ok(b) => b,
                        Err(ub) => return ExecutionOutcome::Stuck { reason: ub, cost: self.cost },
                    };
                    let target = if b { *then_block } else { *else_block };
                    self.trace.record_branch(current_block_id, target);
                    current_block_id = target;
                }
                WirTerminator::Trap(msg) => {
                    return ExecutionOutcome::Trap {
                        reason: msg.clone(),
                        cost: self.cost,
                    };
                }
                WirTerminator::Unreachable => {
                    return ExecutionOutcome::Stuck {
                        reason: UndefinedBehavior::UnreachableHit,
                        cost: self.cost,
                    };
                }
            }
        }
    }

    fn eval_expr(
        &self,
        expr: &WirExpr,
        env: &HashMap<VarId, WirValue>,
    ) -> Result<WirValue, UndefinedBehavior> {
        match expr {
            WirExpr::ConstInt { val, ty } => Ok(WirValue::Int {
                val: *val,
                ty: ty.clone(),
            }),
            WirExpr::ConstBool(b) => Ok(WirValue::Bool(*b)),
            WirExpr::ConstBytes(bytes) => Ok(WirValue::Bytes(bytes.clone())),
            WirExpr::ConstUnit => Ok(WirValue::Unit),
            WirExpr::Var(var) => env
                .get(var)
                .cloned()
                .ok_or_else(|| UndefinedBehavior::TypeMismatch(format!("Undefined var {var:?}"))),
            WirExpr::Unary { op, expr } => {
                let inner = self.eval_expr(expr, env)?;
                match op {
                    UnaryOp::Not => match inner {
                        WirValue::Bool(b) => Ok(WirValue::Bool(!b)),
                        WirValue::Int { val, ty } => {
                            let mask = ty.max_value().unwrap_or(0);
                            Ok(WirValue::Int {
                                val: (!val) & mask,
                                ty,
                            })
                        }
                        _ => Err(UndefinedBehavior::TypeMismatch("Not operand invalid".to_string())),
                    },
                    UnaryOp::Neg(mode) => {
                        let (val, ty) = inner.as_int()?;
                        let res = match mode {
                            OverflowMode::Wrap => {
                                let min = ty.min_value().unwrap_or(0);
                                let max = ty.max_value().unwrap_or(0);
                                if val == min && ty.is_signed_integer() {
                                    min
                                } else {
                                    (-val).clamp(min, max)
                                }
                            }
                            OverflowMode::Trap => {
                                let min = ty.min_value().unwrap_or(0);
                                if val == min && ty.is_signed_integer() {
                                    return Err(UndefinedBehavior::TypeMismatch("Neg overflow trap".to_string()));
                                }
                                -val
                            }
                            OverflowMode::Saturate => {
                                let max = ty.max_value().unwrap_or(0);
                                let min = ty.min_value().unwrap_or(0);
                                (-val).clamp(min, max)
                            }
                        };
                        Ok(WirValue::Int { val: res, ty: ty.clone() })
                    }
                }
            }
            WirExpr::Binary { op, lhs, rhs } => {
                let lval = self.eval_expr(lhs, env)?;
                let rval = self.eval_expr(rhs, env)?;
                eval_binary(*op, &lval, &rval)
            }
            WirExpr::Trunc { expr, target_ty } => {
                let (val, _) = self.eval_expr(expr, env)?.as_int()?;
                let mask = target_ty.max_value().unwrap_or(0);
                Ok(WirValue::Int {
                    val: val & mask,
                    ty: target_ty.clone(),
                })
            }
            WirExpr::ZExt { expr, target_ty } => {
                let inner_val = self.eval_expr(expr, env)?;
                let val = match inner_val {
                    WirValue::Bool(b) => {
                        if b {
                            1
                        } else {
                            0
                        }
                    }
                    WirValue::Int { val, ty: from_ty } => {
                        let mask = from_ty.max_value().unwrap_or(0);
                        val & mask
                    }
                    _ => {
                        return Err(UndefinedBehavior::TypeMismatch(
                            "Expected integer or bool for ZExt".to_string(),
                        ))
                    }
                };
                Ok(WirValue::Int {
                    val,
                    ty: target_ty.clone(),
                })
            }
            WirExpr::SExt { expr, target_ty } => {
                let inner_val = self.eval_expr(expr, env)?;
                match inner_val {
                    WirValue::Bool(b) => {
                        let val = if b { -1i128 } else { 0i128 };
                        let mask = target_ty.max_value().unwrap_or(0);
                        Ok(WirValue::Int {
                            val: val & mask,
                            ty: target_ty.clone(),
                        })
                    }
                    WirValue::Int { val, ty: from_ty } => {
                        let width = from_ty.bit_width().unwrap_or(64);
                        let sign_bit = 1i128 << (width - 1);
                        let extended = if (val & sign_bit) != 0 {
                            let ones = !((1i128 << width) - 1);
                            val | ones
                        } else {
                            val
                        };
                        Ok(WirValue::Int {
                            val: extended,
                            ty: target_ty.clone(),
                        })
                    }
                    _ => Err(UndefinedBehavior::TypeMismatch(
                        "Expected integer or bool for SExt".to_string(),
                    )),
                }
            }
            WirExpr::TupleMake(elems) => {
                let mut vals = Vec::new();
                for e in elems {
                    vals.push(self.eval_expr(e, env)?);
                }
                Ok(WirValue::Tuple(vals))
            }
            WirExpr::TupleGet { expr, index } => {
                let val = self.eval_expr(expr, env)?;
                match val {
                    WirValue::Tuple(elems) => elems
                        .get(*index)
                        .cloned()
                        .ok_or_else(|| UndefinedBehavior::TypeMismatch("Tuple index out of bounds".to_string())),
                    _ => Err(UndefinedBehavior::TypeMismatch("Expected tuple".to_string())),
                }
            }
            WirExpr::PtrOffset { ptr, offset_bytes } => {
                let (prov, base_offset) = self.eval_expr(ptr, env)?.as_ptr()?;
                let (delta, _) = self.eval_expr(offset_bytes, env)?.as_int()?;
                let new_offset = if delta >= 0 {
                    base_offset.saturating_add(delta as u64)
                } else {
                    base_offset.saturating_sub((-delta) as u64)
                };
                Ok(WirValue::Ptr {
                    provenance: prov,
                    offset: new_offset,
                })
            }
            WirExpr::PtrLen(ptr) => {
                let (prov, _) = self.eval_expr(ptr, env)?.as_ptr()?;
                let alloc = self
                    .memory
                    .allocations
                    .get(&prov)
                    .ok_or(UndefinedBehavior::InvalidProvenance(prov))?;
                Ok(WirValue::Int {
                    val: alloc.size as i128,
                    ty: WirType::U64,
                })
            }
            WirExpr::MakeDynamic { expr, tag } => {
                let inner = self.eval_expr(expr, env)?;
                Ok(WirValue::Dynamic {
                    tag: tag.clone(),
                    inner: Box::new(inner),
                })
            }
            WirExpr::CheckDynamic { expr, expected_tag } => {
                let val = self.eval_expr(expr, env)?;
                match val {
                    WirValue::Dynamic { tag, inner } => {
                        if &tag == expected_tag {
                            Ok(*inner)
                        } else {
                            Err(UndefinedBehavior::DynamicTagMismatch {
                                expected: expected_tag.clone(),
                                actual: tag,
                            })
                        }
                    }
                    _ => Err(UndefinedBehavior::TypeMismatch("Expected dynamic value".to_string())),
                }
            }
        }
    }
}

fn eval_binary(
    op: BinaryOp,
    lhs: &WirValue,
    rhs: &WirValue,
) -> Result<WirValue, UndefinedBehavior> {
    match op {
        BinaryOp::Eq => {
            if let (Ok((l, _)), Ok((r, _))) = (lhs.as_int(), rhs.as_int()) {
                Ok(WirValue::Bool(l == r))
            } else {
                Ok(WirValue::Bool(lhs == rhs))
            }
        }
        BinaryOp::Ne => {
            if let (Ok((l, _)), Ok((r, _))) = (lhs.as_int(), rhs.as_int()) {
                Ok(WirValue::Bool(l != r))
            } else {
                Ok(WirValue::Bool(lhs != rhs))
            }
        }
        BinaryOp::Lt => {
            let (l, _) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            Ok(WirValue::Bool(l < r))
        }
        BinaryOp::Le => {
            let (l, _) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            Ok(WirValue::Bool(l <= r))
        }
        BinaryOp::Gt => {
            let (l, _) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            Ok(WirValue::Bool(l > r))
        }
        BinaryOp::Ge => {
            let (l, _) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            Ok(WirValue::Bool(l >= r))
        }
        BinaryOp::BitAnd => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            Ok(WirValue::Int { val: l & r, ty: ty.clone() })
        }
        BinaryOp::BitOr => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            Ok(WirValue::Int { val: l | r, ty: ty.clone() })
        }
        BinaryOp::BitXor => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            Ok(WirValue::Int { val: l ^ r, ty: ty.clone() })
        }
        BinaryOp::Shl => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            if !(0..128).contains(&r) {
                return Err(UndefinedBehavior::ShiftOverflow);
            }
            let mask = ty.max_value().unwrap_or(0);
            let res = (l << (r as u32)) & mask;
            Ok(WirValue::Int { val: res, ty: ty.clone() })
        }
        BinaryOp::Shr => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            if !(0..128).contains(&r) {
                return Err(UndefinedBehavior::ShiftOverflow);
            }
            let res = l >> (r as u32);
            Ok(WirValue::Int { val: res, ty: ty.clone() })
        }
        BinaryOp::Add(mode) => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            let min = ty.min_value().unwrap_or(0);
            let max = ty.max_value().unwrap_or(0);
            let raw = l + r;
            let val = apply_overflow(raw, min, max, mode)?;
            Ok(WirValue::Int { val, ty: ty.clone() })
        }
        BinaryOp::Sub(mode) => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            let min = ty.min_value().unwrap_or(0);
            let max = ty.max_value().unwrap_or(0);
            let raw = l - r;
            let val = apply_overflow(raw, min, max, mode)?;
            Ok(WirValue::Int { val, ty: ty.clone() })
        }
        BinaryOp::Mul(mode) => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            let min = ty.min_value().unwrap_or(0);
            let max = ty.max_value().unwrap_or(0);
            let raw = l * r;
            let val = apply_overflow(raw, min, max, mode)?;
            Ok(WirValue::Int { val, ty: ty.clone() })
        }
        BinaryOp::Div(mode) => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            if r == 0 {
                return Err(UndefinedBehavior::DivisionByZero);
            }
            let min = ty.min_value().unwrap_or(0);
            let max = ty.max_value().unwrap_or(0);
            let raw = l / r;
            let val = apply_overflow(raw, min, max, mode)?;
            Ok(WirValue::Int { val, ty: ty.clone() })
        }
        BinaryOp::Rem(mode) => {
            let (l, ty) = lhs.as_int()?;
            let (r, _) = rhs.as_int()?;
            if r == 0 {
                return Err(UndefinedBehavior::DivisionByZero);
            }
            let min = ty.min_value().unwrap_or(0);
            let max = ty.max_value().unwrap_or(0);
            let raw = l % r;
            let val = apply_overflow(raw, min, max, mode)?;
            Ok(WirValue::Int { val, ty: ty.clone() })
        }
    }
}

fn apply_overflow(
    raw: i128,
    min: i128,
    max: i128,
    mode: OverflowMode,
) -> Result<i128, UndefinedBehavior> {
    match mode {
        OverflowMode::Wrap => {
            let range = max - min + 1;
            if range <= 0 {
                return Ok(raw);
            }
            let mut res = (raw - min) % range;
            if res < 0 {
                res += range;
            }
            Ok(res + min)
        }
        OverflowMode::Trap => {
            if raw < min || raw > max {
                Err(UndefinedBehavior::TypeMismatch("Arithmetic overflow trap".to_string()))
            } else {
                Ok(raw)
            }
        }
        OverflowMode::Saturate => Ok(raw.clamp(min, max)),
    }
}

fn encode_value(val: &WirValue) -> Result<Vec<u8>, UndefinedBehavior> {
    match val {
        WirValue::Bool(b) => Ok(vec![if *b { 1 } else { 0 }]),
        WirValue::Int { val, ty } => match ty {
            WirType::U8 | WirType::I8 => Ok(vec![*val as u8]),
            WirType::U16 | WirType::I16 => Ok((*val as u16).to_le_bytes().to_vec()),
            WirType::U32 | WirType::I32 => Ok((*val as u32).to_le_bytes().to_vec()),
            WirType::U64 | WirType::I64 => Ok((*val as u64).to_le_bytes().to_vec()),
            _ => Err(UndefinedBehavior::TypeMismatch("Unsupported int encoding".to_string())),
        },
        WirValue::Ptr { offset, .. } => Ok(offset.to_le_bytes().to_vec()),
        WirValue::Bytes(b) => Ok(b.clone()),
        _ => Err(UndefinedBehavior::TypeMismatch("Cannot encode value into raw bytes".to_string())),
    }
}

fn decode_value(bytes: &[u8], ty: &WirType) -> Result<WirValue, UndefinedBehavior> {
    match ty {
        WirType::Bool => {
            if bytes.is_empty() {
                return Err(UndefinedBehavior::TypeMismatch("Empty bytes for bool".to_string()));
            }
            Ok(WirValue::Bool(bytes[0] != 0))
        }
        WirType::U8 => {
            if bytes.is_empty() {
                return Err(UndefinedBehavior::TypeMismatch("Empty bytes for u8".to_string()));
            }
            Ok(WirValue::Int { val: bytes[0] as i128, ty: WirType::U8 })
        }
        WirType::I8 => {
            if bytes.is_empty() {
                return Err(UndefinedBehavior::TypeMismatch("Empty bytes for i8".to_string()));
            }
            Ok(WirValue::Int { val: (bytes[0] as i8) as i128, ty: WirType::I8 })
        }
        WirType::U16 => {
            if bytes.len() < 2 {
                return Err(UndefinedBehavior::TypeMismatch("Not enough bytes for u16".to_string()));
            }
            let val = u16::from_le_bytes([bytes[0], bytes[1]]) as i128;
            Ok(WirValue::Int { val, ty: WirType::U16 })
        }
        WirType::I16 => {
            if bytes.len() < 2 {
                return Err(UndefinedBehavior::TypeMismatch("Not enough bytes for i16".to_string()));
            }
            let val = i16::from_le_bytes([bytes[0], bytes[1]]) as i128;
            Ok(WirValue::Int { val, ty: WirType::I16 })
        }
        WirType::U32 => {
            if bytes.len() < 4 {
                return Err(UndefinedBehavior::TypeMismatch("Not enough bytes for u32".to_string()));
            }
            let val = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i128;
            Ok(WirValue::Int { val, ty: WirType::U32 })
        }
        WirType::I32 => {
            if bytes.len() < 4 {
                return Err(UndefinedBehavior::TypeMismatch("Not enough bytes for i32".to_string()));
            }
            let val = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i128;
            Ok(WirValue::Int { val, ty: WirType::I32 })
        }
        WirType::U64 => {
            if bytes.len() < 8 {
                return Err(UndefinedBehavior::TypeMismatch("Not enough bytes for u64".to_string()));
            }
            let arr = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]];
            let val = u64::from_le_bytes(arr) as i128;
            Ok(WirValue::Int { val, ty: WirType::U64 })
        }
        WirType::I64 => {
            if bytes.len() < 8 {
                return Err(UndefinedBehavior::TypeMismatch("Not enough bytes for i64".to_string()));
            }
            let arr = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]];
            let val = i64::from_le_bytes(arr) as i128;
            Ok(WirValue::Int { val, ty: WirType::I64 })
        }
        WirType::ByteSlice => Ok(WirValue::Bytes(bytes.to_vec())),
        WirType::Ptr(_) => {
            if bytes.len() < 8 {
                return Err(UndefinedBehavior::TypeMismatch("Not enough bytes for ptr".to_string()));
            }
            let arr = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]];
            let offset = u64::from_le_bytes(arr);
            Ok(WirValue::Ptr {
                provenance: ProvenanceId(1),
                offset,
            })
        }
        _ => Err(UndefinedBehavior::TypeMismatch("Unsupported decode type".to_string())),
    }
}
