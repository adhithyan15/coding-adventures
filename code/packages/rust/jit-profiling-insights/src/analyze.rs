//! [`analyze`] — the main entry point for jit-profiling-insights.
//!
//! This module ties together the three sub-passes:
//!
//! 1. **Scan** — iterate over every instruction in every function.
//! 2. **Classify** — call [`classify_cost`] and [`find_root_register`]
//!    for each instruction to determine dispatch overhead and root cause.
//! 3. **Rank** — sort the resulting [`TypeSite`] list by impact and wrap it
//!    in a [`ProfilingReport`].
//!
//! The function is intentionally stateless — it reads the IIR annotations
//! written by vm-core and jit-core but never modifies them.  The same
//! `fn_list` can be passed to `analyze` multiple times.
//!
//! # Example
//!
//! ```
//! use jit_profiling_insights::analyze;
//! use interpreter_ir::{IIRFunction, IIRInstr};
//!
//! let fn_ = IIRFunction::new("main", vec![], "void", vec![]);
//! let report = analyze(&[fn_], "my_program", 1);
//! assert_eq!(report.program_name, "my_program");
//! assert!(report.sites.is_empty());
//! ```

use interpreter_ir::function::IIRFunction;

use crate::classify::{classify_cost, find_root_register, savings_description};
use crate::rank::{rank_sites, total_instructions};
use crate::types::{DispatchCost, ProfilingReport, TypeSite};

/// Analyse a list of profiled IIR functions and produce a [`ProfilingReport`].
///
/// The algorithm runs in four steps:
///
/// **Step 1 — Compute total instruction count**
/// Sum `observation_count` across all instructions.  Used for percentage
/// estimates in the text report.
///
/// **Step 2 — Scan every instruction**
/// For each instruction in each function that the profiler observed
/// (`observation_count >= min_call_count`), classify its dispatch cost
/// and, for non-NONE costs, find the root untyped register.
///
/// **Step 3 — Build TypeSite records**
/// For each non-NONE site, construct a [`TypeSite`] with the classified
/// cost, root register, and a human-readable savings description.
///
/// **Step 4 — Rank and wrap**
/// Sort sites by descending impact and return a [`ProfilingReport`].
///
/// # Parameters
///
/// - `fn_list` — `IIRFunction` objects whose instructions carry profiler
///   annotations (`observed_type`, `observation_count`).
/// - `program_name` — a friendly label for the report.
/// - `min_call_count` — instructions with fewer than this many observations
///   are skipped.  Default 1 means "include all observed instructions".
pub fn analyze(
    fn_list: &[IIRFunction],
    program_name: &str,
    min_call_count: u32,
) -> ProfilingReport {
    // Step 1 — total instruction count for percentage calculations.
    let total = total_instructions(fn_list);

    let mut sites: Vec<TypeSite> = Vec::new();

    // Steps 2 & 3 — scan, classify, build TypeSite records.
    for fn_ in fn_list {
        for (idx, instr) in fn_.instructions.iter().enumerate() {
            // Skip instructions the profiler never reached.
            if instr.observation_count < min_call_count {
                continue;
            }

            let cost = classify_cost(instr);

            // NONE means no overhead — skip it; keep the report focused.
            if cost == DispatchCost::None {
                continue;
            }

            // Trace the data-flow chain to find the root untyped register.
            let root_reg = find_root_register(instr, &fn_.instructions, idx);

            let observed = instr.observed_type.clone().unwrap_or_else(|| "unknown".into());
            let deopt_count: u64 = if instr.deopt_anchor.is_some() {
                instr.observation_count as u64
            } else {
                0
            };

            let savings = savings_description(cost, instr.observation_count as u64, &instr.op);

            sites.push(TypeSite {
                function: fn_.name.clone(),
                instruction_op: instr.op.clone(),
                source_register: root_reg,
                observed_type: observed,
                type_hint: instr.type_hint.clone(),
                dispatch_cost: cost,
                call_count: instr.observation_count as u64,
                deopt_count,
                savings_description: savings,
            });
        }
    }

    // Step 4 — rank and wrap.
    rank_sites(&mut sites);

    ProfilingReport {
        program_name: program_name.to_string(),
        total_instructions_executed: total,
        sites,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, Operand};

    fn make_fn_with_type_assert(call_count: u32) -> IIRFunction {
        // A function with one "any"-typed type_assert instruction (Guard cost).
        let mut instr = IIRInstr::new(
            "type_assert",
            None,
            vec![Operand::Var("r0".into())],
            "any",
        );
        instr.observation_count = call_count;
        IIRFunction::new("add", vec![], "any", vec![instr])
    }

    fn make_fn_with_typed_instr() -> IIRFunction {
        let mut instr = IIRInstr::new("add", Some("v0".into()), vec![], "u8");
        instr.observation_count = 100;
        IIRFunction::new("typed_add", vec![], "u8", vec![instr])
    }

    #[test]
    fn analyze_empty_fn_list() {
        let report = analyze(&[], "empty", 1);
        assert_eq!(report.program_name, "empty");
        assert_eq!(report.total_instructions_executed, 0);
        assert!(report.sites.is_empty());
    }

    #[test]
    fn analyze_skips_typed_instructions() {
        let fn_ = make_fn_with_typed_instr();
        let report = analyze(&[fn_], "test", 1);
        assert!(report.sites.is_empty());
    }

    #[test]
    fn analyze_captures_guard_site() {
        let fn_ = make_fn_with_type_assert(1_000);
        let report = analyze(&[fn_], "test", 1);
        assert_eq!(report.sites.len(), 1);
        assert_eq!(report.sites[0].dispatch_cost, DispatchCost::Guard);
        assert_eq!(report.sites[0].call_count, 1_000);
        assert_eq!(report.sites[0].function, "add");
    }

    #[test]
    fn analyze_min_call_count_filters() {
        let fn_ = make_fn_with_type_assert(5);
        let report = analyze(&[fn_], "test", 10); // threshold 10 > actual 5
        assert!(report.sites.is_empty());
    }

    #[test]
    fn analyze_multiple_fns_ranked() {
        let fn1 = make_fn_with_type_assert(100);   // impact 100
        let fn2 = make_fn_with_type_assert(1_000); // impact 1000
        let report = analyze(&[fn1, fn2], "test", 1);
        assert_eq!(report.sites.len(), 2);
        // highest impact first
        assert!(report.sites[0].call_count >= report.sites[1].call_count);
    }

    #[test]
    fn analyze_total_instructions_correct() {
        let fn_ = make_fn_with_type_assert(42);
        let report = analyze(&[fn_], "test", 1);
        assert_eq!(report.total_instructions_executed, 42);
    }

    #[test]
    fn analyze_captures_generic_call_site() {
        let mut instr = IIRInstr::new(
            "call_runtime",
            Some("v0".into()),
            vec![Operand::Var("generic_add".into())],
            "any",
        );
        instr.observation_count = 200;
        let fn_ = IIRFunction::new("f", vec![], "any", vec![instr]);
        let report = analyze(&[fn_], "test", 1);
        assert_eq!(report.sites[0].dispatch_cost, DispatchCost::GenericCall);
    }

    #[test]
    fn analyze_program_name_propagated() {
        let report = analyze(&[], "my_program", 1);
        assert_eq!(report.program_name, "my_program");
    }
}
