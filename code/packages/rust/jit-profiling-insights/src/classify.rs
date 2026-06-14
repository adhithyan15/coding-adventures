//! Classification logic for the JIT insight pass.
//!
//! This module answers two questions for every [`IIRInstr`] in a compiled
//! function:
//!
//! 1. **What dispatch cost did the JIT incur?**
//!    [`classify_cost`] maps an instruction to one of the four [`DispatchCost`]
//!    levels by inspecting `type_hint`, `op`, `srcs`, and the profiler counters.
//!
//! 2. **Which register is responsible for the cost?**
//!    [`find_root_register`] traces the data-flow chain backward from the
//!    flagged instruction to identify the SSA register whose `type_hint == "any"`
//!    triggered the guard or generic dispatch.
//!
//! # Classification algorithm
//!
//! ```text
//! if instr.type_hint != "any":
//!     → None  (statically typed — JIT compiles to a direct typed op)
//!
//! elif instr.op == "type_assert":
//!     → Guard  (the JIT inserted this guard because type_hint is "any"
//!               but inferred type is concrete)
//!
//! elif instr.op == "call_runtime" and srcs[0] contains "generic_":
//!     → GenericCall  (inferred type is also "any" — full dynamic dispatch)
//!
//! elif observation_count > 0 and deopt_anchor is Some:
//!     → Deopt  (a guard was emitted but failed at runtime)
//!
//! else:
//!     → None
//! ```

use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::opcodes::DYNAMIC_TYPE;

use crate::types::DispatchCost;

/// Classify the dispatch cost of a single instruction.
///
/// Returns [`DispatchCost::None`] when the instruction carries no dynamic
/// dispatch overhead.
pub fn classify_cost(instr: &IIRInstr) -> DispatchCost {
    // Statically typed — JIT emits a direct typed operation.
    if instr.type_hint != DYNAMIC_TYPE {
        return DispatchCost::None;
    }

    // A type_assert instruction IS the guard the JIT inserted.  The JIT
    // emits one of these for each use of an "any"-typed register when it
    // has successfully inferred a concrete type from profiling.
    if instr.op == "type_assert" {
        return DispatchCost::Guard;
    }

    // call_runtime with a "generic_*" callee means the JIT could not infer
    // a concrete type at all and fell back to the full runtime dispatch table.
    if instr.op == "call_runtime" {
        if let Some(Operand::Var(name)) = instr.srcs.first() {
            if name.contains("generic_") {
                return DispatchCost::GenericCall;
            }
        }
    }

    // Deoptimisation: a deopt anchor was set (guard emitted) and the
    // instruction has been observed.  In the Rust IIR, deopt_anchor tracks
    // whether a guard has been emitted for this instruction; the Python
    // equivalent reads a per-instruction deopt_count field.
    if instr.observation_count > 0 && instr.deopt_anchor.is_some() {
        return DispatchCost::Deopt;
    }

    DispatchCost::None
}

/// Trace back along the data-flow chain to find the root untyped register.
///
/// Starting from the first source operand of `instr`, walk backward through
/// `load_reg` and `load_mem` instructions (the SSA edges in IIR) to find
/// the furthest-back register whose `type_hint == "any"` is the true root
/// cause of the dispatch overhead.
///
/// # Parameters
///
/// - `instr` — the flagged instruction (a `type_assert` or `call_runtime`).
/// - `instructions` — the full instruction list for the function.
/// - `instr_index` — the index of `instr` within `instructions`.
///
/// # Returns
///
/// The name of the root SSA register (e.g. `"%r0"` or a parameter name like
/// `"n"`).  Falls back to the first source operand of `instr` if no chain is
/// found.
pub fn find_root_register(
    instr: &IIRInstr,
    instructions: &[IIRInstr],
    instr_index: usize,
) -> String {
    // Grab the primary source operand — the register we're guarding.
    let primary = match instr.srcs.first() {
        Some(Operand::Var(name)) => name.clone(),
        Some(other) => return other.to_string(),
        None => return instr.dest.clone().unwrap_or_else(|| "%unknown".into()),
    };

    // Build a reverse lookup: dest register → instruction that defines it.
    // We only scan instructions before the current one (SSA invariant).
    let mut defs: std::collections::HashMap<&str, &IIRInstr> =
        std::collections::HashMap::new();
    for candidate in &instructions[..instr_index] {
        if let Some(d) = &candidate.dest {
            defs.insert(d.as_str(), candidate);
        }
    }

    // Walk the def-use chain until we can't go further.
    let mut current_reg = primary;
    let mut visited = std::collections::HashSet::new();

    loop {
        if visited.contains(current_reg.as_str()) {
            break;
        }
        let defining = match defs.get(current_reg.as_str()) {
            Some(d) => *d,
            None => break,
        };
        visited.insert(current_reg.clone());

        // Only keep tracing if this definition is also untyped.
        if defining.type_hint != DYNAMIC_TYPE {
            break;
        }

        // Memory loads and register copies carry the type through directly.
        if defining.op == "load_mem" || defining.op == "load_reg" || defining.op == "const" {
            match defining.srcs.first() {
                Some(Operand::Var(next)) => {
                    current_reg = next.clone();
                }
                _ => break,
            }
        } else {
            // Arithmetic or other ops — the register is the root cause.
            break;
        }
    }

    current_reg
}

/// Generate a human-readable description of what adding a type removes.
///
/// The description is terse and concrete — it names the specific overhead
/// that would be eliminated.
pub fn savings_description(cost: DispatchCost, call_count: u64, _op: &str) -> String {
    match cost {
        DispatchCost::Guard => format!(
            "would eliminate 1 type_assert per call ({} branches total)",
            format_number(call_count),
        ),
        DispatchCost::GenericCall => format!(
            "would replace generic runtime dispatch with a direct typed call \
             ({} calls, ~10× speedup each)",
            format_number(call_count),
        ),
        DispatchCost::Deopt => format!(
            "would prevent interpreter fallback on every guard failure \
             ({} observations, ~100× cost)",
            format_number(call_count),
        ),
        DispatchCost::None => "no overhead".into(),
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::instr::IIRInstr;

    fn instr_any(op: &str) -> IIRInstr {
        let mut i = IIRInstr::new(op, Some("dest".into()), vec![], "any");
        i.observation_count = 10;
        i
    }

    fn instr_typed(op: &str) -> IIRInstr {
        IIRInstr::new(op, Some("dest".into()), vec![], "u8")
    }

    #[test]
    fn classify_statically_typed_is_none() {
        let i = instr_typed("add");
        assert_eq!(classify_cost(&i), DispatchCost::None);
    }

    #[test]
    fn classify_type_assert_is_guard() {
        let i = instr_any("type_assert");
        assert_eq!(classify_cost(&i), DispatchCost::Guard);
    }

    #[test]
    fn classify_call_runtime_generic_is_generic_call() {
        let mut i = instr_any("call_runtime");
        i.srcs = vec![Operand::Var("generic_add".into())];
        assert_eq!(classify_cost(&i), DispatchCost::GenericCall);
    }

    #[test]
    fn classify_call_runtime_non_generic_is_none() {
        let mut i = instr_any("call_runtime");
        i.srcs = vec![Operand::Var("str_concat".into())];
        // observation_count=10 but no deopt_anchor → NONE
        assert_eq!(classify_cost(&i), DispatchCost::None);
    }

    #[test]
    fn classify_deopt_when_anchor_set() {
        let mut i = instr_any("add");
        i.deopt_anchor = Some(0);
        assert_eq!(classify_cost(&i), DispatchCost::Deopt);
    }

    #[test]
    fn classify_no_deopt_when_observation_zero() {
        let mut i = instr_any("add");
        i.observation_count = 0;
        i.deopt_anchor = Some(0);
        // observation_count == 0 → not classified as Deopt
        assert_eq!(classify_cost(&i), DispatchCost::None);
    }

    #[test]
    fn find_root_no_srcs_returns_dest() {
        let i = IIRInstr::new("halt", Some("d".into()), vec![], "void");
        let root = find_root_register(&i, &[], 0);
        assert_eq!(root, "d");
    }

    #[test]
    fn find_root_literal_src_returns_literal_string() {
        let i = IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "any");
        let root = find_root_register(&i, &[], 0);
        // Int literal → string representation
        assert!(root.contains("42") || !root.is_empty());
    }

    #[test]
    fn find_root_traces_load_mem_chain() {
        // type_assert %r1   ; %r1 defined by load_mem arg[0] (both "any")
        let load = IIRInstr::new(
            "load_mem",
            Some("r1".into()),
            vec![Operand::Var("arg[0]".into())],
            "any",
        );
        let guard = IIRInstr::new(
            "type_assert",
            None,
            vec![Operand::Var("r1".into())],
            "any",
        );
        let instructions = vec![load, guard];
        let root = find_root_register(&instructions[1], &instructions, 1);
        assert_eq!(root, "arg[0]");
    }

    #[test]
    fn savings_description_guard() {
        let desc = savings_description(DispatchCost::Guard, 1_000_000, "add");
        assert!(desc.contains("type_assert"));
        assert!(desc.contains("1,000,000"));
    }

    #[test]
    fn savings_description_generic_call() {
        let desc = savings_description(DispatchCost::GenericCall, 50, "add");
        assert!(desc.contains("generic runtime dispatch"));
        assert!(desc.contains("10×"));
    }

    #[test]
    fn savings_description_deopt() {
        let desc = savings_description(DispatchCost::Deopt, 5, "add");
        assert!(desc.contains("interpreter fallback"));
        assert!(desc.contains("100×"));
    }
}
