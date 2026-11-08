//! Warrant Intermediate Representation (WIR) core AST, type checker, cost semantics,
//! observable event trace, and reference interpreter (VER00 §7.5.5, VER25, VER-P021).

pub mod ast;
pub mod check;
pub mod constant_time;
pub mod cost;
pub mod interp;
pub mod trace;

pub use ast::*;
pub use check::{TypeChecker, TypeError};
pub use constant_time::{check_constant_time, check_constant_time_default, ConstantTimeVerdict};
pub use cost::{CostSchedule, ExecutionCost};
pub use interp::{
    Allocation, BudgetLimit, CapabilityDispatcher, DefaultCapabilityDispatcher, ExecutionOutcome,
    Interpreter, InterpreterConfig, Memory, UndefinedBehavior, WirValue,
};
pub use trace::{AccessKind, ObservableTrace, TraceEvent};
