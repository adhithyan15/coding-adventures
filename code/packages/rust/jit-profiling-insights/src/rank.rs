//! Impact ranking for the JIT insight pass.
//!
//! The ranking formula converts raw profiler numbers into a single integer
//! that lets the report sort sites from "fix this first" to "fix this last":
//!
//! ```text
//! impact = call_count × cost_weight
//! ```
//!
//! where `cost_weight` is [`DispatchCost::weight`].
//!
//! # Why this formula?
//!
//! We do not have cycle-accurate timings per instruction.  `call_count ×
//! cost_weight` is a conservative *proxy* that preserves the relative ordering
//! of dispatch strategies:
//!
//! - A DEOPT site with 10 calls scores 1 000 — worse than a GUARD site with
//!   100 calls (score 100), because deoptimisations are ~100× more expensive.
//! - A GENERIC_CALL site with 50 000 calls scores 500 000 — worse than a GUARD
//!   site with 200 000 calls (score 200 000), even though the GUARD fires more
//!   often, because each generic call is ~10× more expensive.

use interpreter_ir::function::IIRFunction;

use crate::types::{DispatchCost, TypeSite};

/// Sort *sites* in-place by descending impact score.
///
/// Sites with equal impact are further sorted by [`DispatchCost::weight`]
/// (descending) so that DEOPT ties beat GENERIC_CALL ties beat GUARD ties.
/// This ensures the most severe dispatch strategy is always shown first
/// when the raw impact scores happen to be equal.
///
/// Returns a mutable reference to the same slice for call-chaining.
pub fn rank_sites(sites: &mut Vec<TypeSite>) {
    sites.sort_by(|a, b| {
        // Primary: descending impact
        b.impact().cmp(&a.impact())
            // Tie-break: descending cost weight
            .then_with(|| b.dispatch_cost.weight().cmp(&a.dispatch_cost.weight()))
    });
}

/// Sum observation counts across all functions to get total executed instructions.
///
/// Only instructions that the profiler actually sampled (`observation_count > 0`)
/// are counted.  Instructions that were never reached have a count of zero
/// and contribute nothing.
///
/// # Returns
///
/// Total number of instruction executions observed by the profiler.
/// Zero if no profiling data was recorded.
pub fn total_instructions(fn_list: &[IIRFunction]) -> u64 {
    fn_list.iter()
        .flat_map(|f| &f.instructions)
        .map(|i| i.observation_count as u64)
        .sum()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DispatchCost, TypeSite};
    use interpreter_ir::{IIRFunction, IIRInstr, Operand};

    fn make_site(cost: DispatchCost, calls: u64) -> TypeSite {
        TypeSite {
            function: "f".into(),
            instruction_op: "add".into(),
            source_register: "%r0".into(),
            observed_type: "u8".into(),
            type_hint: "any".into(),
            dispatch_cost: cost,
            call_count: calls,
            deopt_count: 0,
            savings_description: "test".into(),
        }
    }

    #[test]
    fn rank_sites_descending_impact() {
        let mut sites = vec![
            make_site(DispatchCost::Guard, 100),        // impact 100
            make_site(DispatchCost::Deopt, 10),          // impact 1000
            make_site(DispatchCost::GenericCall, 50),    // impact 500
        ];
        rank_sites(&mut sites);
        assert_eq!(sites[0].dispatch_cost, DispatchCost::Deopt);
        assert_eq!(sites[1].dispatch_cost, DispatchCost::GenericCall);
        assert_eq!(sites[2].dispatch_cost, DispatchCost::Guard);
    }

    #[test]
    fn rank_sites_tiebreak_by_weight() {
        // Both have impact 100, but Deopt has higher weight
        let mut sites = vec![
            make_site(DispatchCost::Guard, 100),   // impact=100, weight=1
            make_site(DispatchCost::Deopt, 1),     // impact=100, weight=100
        ];
        rank_sites(&mut sites);
        assert_eq!(sites[0].dispatch_cost, DispatchCost::Deopt);
    }

    #[test]
    fn rank_sites_empty_ok() {
        let mut sites: Vec<TypeSite> = vec![];
        rank_sites(&mut sites);
        assert!(sites.is_empty());
    }

    #[test]
    fn total_instructions_sums_counts() {
        let mut instr1 = IIRInstr::new("add", Some("v0".into()), vec![], "u8");
        instr1.observation_count = 100;
        let mut instr2 = IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8");
        instr2.observation_count = 50;
        let fn_ = IIRFunction::new("f", vec![], "u8", vec![instr1, instr2]);
        assert_eq!(total_instructions(&[fn_]), 150);
    }

    #[test]
    fn total_instructions_empty() {
        assert_eq!(total_instructions(&[]), 0);
    }

    #[test]
    fn total_instructions_unobserved_zero() {
        // observation_count defaults to 0
        let instr = IIRInstr::new("add", Some("v0".into()), vec![], "any");
        let fn_ = IIRFunction::new("f", vec![], "any", vec![instr]);
        assert_eq!(total_instructions(&[fn_]), 0);
    }
}
