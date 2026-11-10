//! Self-composition constant-time validation over WIR observable traces (VER26 §8.1, CWE-208).

use crate::ast::Program;
use crate::interp::{CapabilityDispatcher, DefaultCapabilityDispatcher, ExecutionOutcome, Interpreter, InterpreterConfig, WirValue};
use crate::trace::TraceEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantTimeVerdict {
    ConstantTime {
        steps: u64,
        trace_events: usize,
    },
    BranchDivergence {
        event_index: usize,
        trace_a: TraceEvent,
        trace_b: TraceEvent,
    },
    MemoryAddressDivergence {
        event_index: usize,
        trace_a: TraceEvent,
        trace_b: TraceEvent,
    },
    CapabilityDivergence {
        event_index: usize,
        trace_a: TraceEvent,
        trace_b: TraceEvent,
    },
    TraceLengthDivergence {
        len_a: usize,
        len_b: usize,
    },
    StepCountDivergence {
        steps_a: u64,
        steps_b: u64,
    },
    ExecutionFailed(String),
}

pub fn check_constant_time<D: CapabilityDispatcher + Clone>(
    program: &Program,
    func_name: &str,
    args_a: Vec<WirValue>,
    args_b: Vec<WirValue>,
    config: InterpreterConfig,
    dispatcher: D,
) -> ConstantTimeVerdict {
    let mut interp_a = Interpreter::with_config(program, config.clone(), dispatcher.clone());
    let outcome_a = interp_a.execute(func_name, args_a);

    let mut interp_b = Interpreter::with_config(program, config, dispatcher);
    let outcome_b = interp_b.execute(func_name, args_b);

    let (cost_a, trace_a) = match outcome_a {
        ExecutionOutcome::Success { cost, trace, .. } => (cost, trace),
        other => return ConstantTimeVerdict::ExecutionFailed(format!("Run A failed: {other:?}")),
    };

    let (cost_b, trace_b) = match outcome_b {
        ExecutionOutcome::Success { cost, trace, .. } => (cost, trace),
        other => return ConstantTimeVerdict::ExecutionFailed(format!("Run B failed: {other:?}")),
    };

    if trace_a.len() != trace_b.len() {
        return ConstantTimeVerdict::TraceLengthDivergence {
            len_a: trace_a.len(),
            len_b: trace_b.len(),
        };
    }

    for (idx, (ev_a, ev_b)) in trace_a.events.iter().zip(&trace_b.events).enumerate() {
        if ev_a != ev_b {
            match (ev_a, ev_b) {
                (TraceEvent::BranchDecision { .. }, TraceEvent::BranchDecision { .. }) => {
                    return ConstantTimeVerdict::BranchDivergence {
                        event_index: idx,
                        trace_a: ev_a.clone(),
                        trace_b: ev_b.clone(),
                    };
                }
                (TraceEvent::MemoryAccess { .. }, TraceEvent::MemoryAccess { .. }) => {
                    return ConstantTimeVerdict::MemoryAddressDivergence {
                        event_index: idx,
                        trace_a: ev_a.clone(),
                        trace_b: ev_b.clone(),
                    };
                }
                _ => {
                    return ConstantTimeVerdict::CapabilityDivergence {
                        event_index: idx,
                        trace_a: ev_a.clone(),
                        trace_b: ev_b.clone(),
                    };
                }
            }
        }
    }

    if cost_a.steps != cost_b.steps {
        return ConstantTimeVerdict::StepCountDivergence {
            steps_a: cost_a.steps,
            steps_b: cost_b.steps,
        };
    }

    ConstantTimeVerdict::ConstantTime {
        steps: cost_a.steps,
        trace_events: trace_a.len(),
    }
}

pub fn check_constant_time_default(
    program: &Program,
    func_name: &str,
    args_a: Vec<WirValue>,
    args_b: Vec<WirValue>,
) -> ConstantTimeVerdict {
    check_constant_time(
        program,
        func_name,
        args_a,
        args_b,
        InterpreterConfig::default(),
        DefaultCapabilityDispatcher,
    )
}
