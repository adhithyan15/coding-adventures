//! [`suggest`] — the main entry point for vm-type-suggestions.
//!
//! # Algorithm
//!
//! For each function in `fn_list`:
//!
//! 1. Skip fully-typed parameters (`type_hint != "any"`).
//! 2. For each untyped parameter at position `N`, find the `load_mem`
//!    instruction whose first source operand is `"arg[N]"`.
//! 3. Classify the observation:
//!    - No instruction found OR `observation_count == 0` → `NoData`
//!    - `observed_type == "polymorphic"` → `Mixed`
//!    - Any other non-None `observed_type` → `Certain`
//! 4. For `Certain`: produce `suggestion = "declare '{param}: {type}'"`.
//! 5. Build a [`ParamSuggestion`] and add it to the report.
//!
//! # Why `load_mem [arg[N]]`?
//!
//! In the IIR produced by gradual-typing language compilers, function arguments
//! are loaded into SSA registers at the very start of the function body via
//! `load_mem` instructions whose source operand names the argument slot:
//!
//! ```text
//! load_mem %r0 <- arg[0] : any
//! load_mem %r1 <- arg[1] : any
//! ```
//!
//! `vm-core`'s profiler calls `instr.record_observation(rt)` after every
//! instruction that produces a value, including these `load_mem` instructions.
//! So after N calls to `add(a, b)`, the `load_mem arg[0]` instruction has
//! `observed_type = Some("u8")` and `observation_count = N` — exactly what
//! we need.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::opcodes::{DYNAMIC_TYPE, POLYMORPHIC_TYPE};

use crate::types::{Confidence, ParamSuggestion, SuggestionReport};

/// Analyse profiled IIR functions and return parameter type suggestions.
///
/// # Parameters
///
/// - `fn_list` — `IIRFunction` objects whose instructions carry profiler
///   annotations (`observed_type`, `observation_count`).
/// - `program_name` — a friendly label for the output report.
///
/// # Returns
///
/// A [`SuggestionReport`] with all untyped parameters classified as
/// `Certain` / `Mixed` / `NoData`.  Use [`.actionable()`] to get only
/// the `Certain` suggestions.
///
/// # Example
///
/// ```
/// use vm_type_suggestions::suggest;
/// use interpreter_ir::{IIRFunction, IIRInstr, Operand};
///
/// // A function with one untyped parameter.
/// let fn_ = IIRFunction::new(
///     "add",
///     vec![("a".into(), "any".into()), ("b".into(), "any".into())],
///     "any",
///     vec![],
/// );
/// let report = suggest(&[fn_], "my_program");
/// assert_eq!(report.suggestions.len(), 2);
/// assert_eq!(report.program_name, "my_program");
/// ```
pub fn suggest(fn_list: &[IIRFunction], program_name: &str) -> SuggestionReport {
    let mut all_suggestions: Vec<ParamSuggestion> = Vec::new();
    let mut total_calls: u64 = 0;

    for fn_ in fn_list {
        // Build a fast lookup: arg_index → first matching load_mem instruction.
        let arg_loaders = find_arg_loaders(fn_);

        for (param_index, (param_name, type_hint)) in fn_.params.iter().enumerate() {
            // Already typed — the compiler knows; no suggestion needed.
            if type_hint != DYNAMIC_TYPE {
                continue;
            }

            let instr = arg_loaders.get(&param_index).copied();

            let suggestion = if instr.map_or(true, |i| i.observation_count == 0) {
                ParamSuggestion {
                    function: fn_.name.clone(),
                    param_name: param_name.clone(),
                    param_index,
                    observed_type: None,
                    call_count: 0,
                    confidence: Confidence::NoData,
                    suggestion: None,
                }
            } else {
                let instr = instr.unwrap();
                let ot = instr.observed_type.as_deref().unwrap_or("");
                if ot == POLYMORPHIC_TYPE {
                    ParamSuggestion {
                        function: fn_.name.clone(),
                        param_name: param_name.clone(),
                        param_index,
                        observed_type: Some(POLYMORPHIC_TYPE.into()),
                        call_count: instr.observation_count as u64,
                        confidence: Confidence::Mixed,
                        suggestion: None,
                    }
                } else {
                    let count = instr.observation_count as u64;
                    total_calls += count;
                    ParamSuggestion {
                        function: fn_.name.clone(),
                        param_name: param_name.clone(),
                        param_index,
                        observed_type: Some(ot.into()),
                        call_count: count,
                        confidence: Confidence::Certain,
                        suggestion: Some(format!("declare '{param_name}: {ot}'")),
                    }
                }
            };

            all_suggestions.push(suggestion);
        }
    }

    SuggestionReport {
        program_name: program_name.to_string(),
        total_calls,
        suggestions: all_suggestions,
    }
}

/// Return a mapping from arg index to the first `load_mem [arg[N]]` instruction.
///
/// Scans all instructions in the function body for `load_mem` instructions
/// whose first source operand matches the pattern `"arg[N]"`.  Returns the
/// first match for each index.
fn find_arg_loaders(fn_: &IIRFunction) -> std::collections::HashMap<usize, &IIRInstr> {
    let mut loaders: std::collections::HashMap<usize, &IIRInstr> =
        std::collections::HashMap::new();

    for instr in &fn_.instructions {
        if instr.op != "load_mem" {
            continue;
        }
        if let Some(Operand::Var(src)) = instr.srcs.first() {
            if src.starts_with("arg[") && src.ends_with(']') {
                if let Ok(idx) = src[4..src.len() - 1].parse::<usize>() {
                    // Keep only the first occurrence (highest observation count).
                    loaders.entry(idx).or_insert(instr);
                }
            }
        }
    }

    loaders
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, Operand};

    /// Build a function with `n` untyped params, each with a `load_mem arg[i]`
    /// instruction whose observation is `counts[i]` (and observed_type is `types[i]`).
    fn make_fn(
        params: &[(&str, &str)],
        observations: &[u32],
        observed_types: &[Option<&str>],
    ) -> IIRFunction {
        let mut instrs = vec![];
        for (i, (_, type_hint)) in params.iter().enumerate() {
            if type_hint == &"any" {
                let src_name = format!("arg[{i}]");
                let mut instr = IIRInstr::new(
                    "load_mem",
                    Some(format!("r{i}")),
                    vec![Operand::Var(src_name)],
                    "any",
                );
                instr.observation_count = *observations.get(i).unwrap_or(&0);
                instr.observed_type = observed_types.get(i).and_then(|o| o.as_ref()).map(|s| s.to_string());
                instrs.push(instr);
            }
        }
        let params_owned: Vec<(String, String)> = params
            .iter()
            .map(|(n, t)| (n.to_string(), t.to_string()))
            .collect();
        IIRFunction::new("add", params_owned, "any", instrs)
    }

    #[test]
    fn suggest_empty_fn_list() {
        let report = suggest(&[], "prog");
        assert!(report.suggestions.is_empty());
        assert_eq!(report.total_calls, 0);
    }

    #[test]
    fn suggest_no_data_when_unobserved() {
        let fn_ = make_fn(&[("n", "any")], &[0], &[None]);
        let report = suggest(&[fn_], "prog");
        assert_eq!(report.suggestions.len(), 1);
        assert_eq!(report.suggestions[0].confidence, Confidence::NoData);
        assert!(report.suggestions[0].suggestion.is_none());
    }

    #[test]
    fn suggest_certain_when_single_type() {
        let fn_ = make_fn(&[("n", "any")], &[1_000], &[Some("u8")]);
        let report = suggest(&[fn_], "prog");
        assert_eq!(report.suggestions.len(), 1);
        assert_eq!(report.suggestions[0].confidence, Confidence::Certain);
        assert_eq!(
            report.suggestions[0].suggestion.as_deref(),
            Some("declare 'n: u8'"),
        );
        assert_eq!(report.suggestions[0].observed_type.as_deref(), Some("u8"));
        assert_eq!(report.total_calls, 1_000);
    }

    #[test]
    fn suggest_mixed_when_polymorphic() {
        let fn_ = make_fn(&[("n", "any")], &[500], &[Some("polymorphic")]);
        let report = suggest(&[fn_], "prog");
        assert_eq!(report.suggestions[0].confidence, Confidence::Mixed);
        assert!(report.suggestions[0].suggestion.is_none());
    }

    #[test]
    fn suggest_skips_typed_params() {
        // param "a" is typed u8 — should be skipped
        let fn_ = make_fn(&[("a", "u8"), ("b", "any")], &[0, 100], &[None, Some("u8")]);
        let report = suggest(&[fn_], "prog");
        // Only "b" should appear
        assert_eq!(report.suggestions.len(), 1);
        assert_eq!(report.suggestions[0].param_name, "b");
    }

    #[test]
    fn suggest_multiple_params() {
        let fn_ = make_fn(
            &[("a", "any"), ("b", "any")],
            &[1_000, 1_000],
            &[Some("u8"), Some("u16")],
        );
        let report = suggest(&[fn_], "prog");
        assert_eq!(report.suggestions.len(), 2);
        assert_eq!(report.suggestions[0].param_name, "a");
        assert_eq!(report.suggestions[1].param_name, "b");
        assert_eq!(report.total_calls, 2_000);
    }

    #[test]
    fn suggest_multiple_functions() {
        let fn1 = make_fn(&[("x", "any")], &[100], &[Some("u8")]);
        let fn2 = make_fn(&[("y", "any")], &[200], &[Some("i32")]);
        let report = suggest(&[fn1, fn2], "prog");
        assert_eq!(report.suggestions.len(), 2);
        assert_eq!(report.total_calls, 300);
    }

    #[test]
    fn suggest_no_load_mem_gives_no_data() {
        // Function has an untyped param but no load_mem instruction for it.
        let fn_ = IIRFunction::new(
            "f",
            vec![("n".into(), "any".into())],
            "any",
            vec![],  // no instructions at all
        );
        let report = suggest(&[fn_], "prog");
        assert_eq!(report.suggestions[0].confidence, Confidence::NoData);
    }

    #[test]
    fn find_arg_loaders_finds_correct_slot() {
        let mut instr0 = IIRInstr::new(
            "load_mem",
            Some("r0".into()),
            vec![Operand::Var("arg[0]".into())],
            "any",
        );
        instr0.observation_count = 5;
        let mut instr1 = IIRInstr::new(
            "load_mem",
            Some("r1".into()),
            vec![Operand::Var("arg[1]".into())],
            "any",
        );
        instr1.observation_count = 7;
        let fn_ = IIRFunction::new("f", vec![], "any", vec![instr0, instr1]);
        let loaders = find_arg_loaders(&fn_);
        assert_eq!(loaders.get(&0).unwrap().observation_count, 5);
        assert_eq!(loaders.get(&1).unwrap().observation_count, 7);
        assert!(!loaders.contains_key(&2));
    }

    #[test]
    fn suggest_program_name_propagated() {
        let report = suggest(&[], "my_program");
        assert_eq!(report.program_name, "my_program");
    }
}
